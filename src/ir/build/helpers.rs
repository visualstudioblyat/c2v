use tree_sitter::Node;
use super::super::DfgOp;

pub(crate) fn extract_loop_bounds(
    init: Option<Node>,
    cond: Option<Node>,
    update: Option<Node>,
    source: &str,
) -> Option<(String, i64, i64)> {
    // Parse: for (int/uint i = START; i < END; i++) or i += 1, i = i + 1
    let init = init?;
    let cond = cond?;

    // Extract variable name and start value from initializer
    // Handles: "int i = 0" or "i = 0" (declaration or assignment)
    let (var_name, start_val) = extract_init(init, source)?;

    // Extract end value from condition: "i < END" or "i <= END"
    let end_val = extract_condition(&var_name, cond, source)?;

    // Verify update increments by 1: "i++" or "i += 1" or "i = i + 1"
    if let Some(upd) = update {
        if !is_increment_by_one(&var_name, upd, source) {
            return None;
        }
    }

    Some((var_name, start_val, end_val))
}

pub(crate) fn extract_init(node: Node, source: &str) -> Option<(String, i64)> {
    let text = node_text(node, source);

    // Try "type var = val" pattern (declaration)
    if node.kind() == "declaration" {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "init_declarator" {
                let decl = child.child_by_field_name("declarator")?;
                let val = child.child_by_field_name("value")?;
                let name = node_text(decl, source).to_string();
                let start = node_text(val, source).trim().parse::<i64>().ok()?;
                return Some((name, start));
            }
        }
    }

    // Try "var = val" pattern (assignment expression)
    if node.kind() == "assignment_expression" {
        let left = node.child_by_field_name("left")?;
        let right = node.child_by_field_name("right")?;
        let name = node_text(left, source).to_string();
        let start = node_text(right, source).trim().parse::<i64>().ok()?;
        return Some((name, start));
    }

    // Try just parsing the text: "int i = 0" or "uint i = 0"
    let parts: Vec<&str> = text.split('=').collect();
    if parts.len() == 2 {
        let lhs = parts[0].trim();
        let rhs = parts[1].trim().trim_end_matches(';');
        let var = lhs.split_whitespace().last()?;
        let val = rhs.parse::<i64>().ok()?;
        return Some((var.to_string(), val));
    }

    None
}

pub(crate) fn extract_condition(var: &str, node: Node, source: &str) -> Option<i64> {
    let text = node_text(node, source).trim().to_string();

    // "i < N" → end = N
    if let Some(rest) = text.strip_prefix(var) {
        let rest = rest.trim();
        if let Some(val_str) = rest.strip_prefix('<') {
            let val_str = val_str.trim().strip_prefix('=').unwrap_or(val_str.trim());
            let val_str = val_str.trim();
            if let Ok(v) = val_str.parse::<i64>() {
                if rest.starts_with("<=") {
                    return Some(v + 1);
                }
                return Some(v);
            }
        }
    }

    // Try via tree-sitter children
    if node.kind() == "binary_expression" {
        let left = node.child_by_field_name("left")?;
        let op = node.child_by_field_name("operator")?;
        let right = node.child_by_field_name("right")?;

        let left_text = node_text(left, source).trim();
        let op_text = node_text(op, source).trim();
        let right_text = node_text(right, source).trim();

        if left_text == var {
            let val = right_text.parse::<i64>().ok()?;
            return match op_text {
                "<" => Some(val),
                "<=" => Some(val + 1),
                ">=" => Some(val),
                ">" => Some(val - 1),
                "!=" => Some(val),
                _ => None,
            };
        }
    }

    None
}

pub(crate) fn is_increment_by_one(var: &str, node: Node, source: &str) -> bool {
    let text = node_text(node, source).trim().to_string();

    // i++ or ++i or i-- or --i
    if text == format!("{var}++") || text == format!("++{var}")
        || text == format!("{var}--") || text == format!("--{var}") {
        return true;
    }
    // i += 1 or i -= 1
    if text == format!("{var} += 1") || text == format!("{var}+=1")
        || text == format!("{var} -= 1") || text == format!("{var}-=1") {
        return true;
    }
    // i = i + 1 or i = i - 1
    if text == format!("{var} = {var} + 1") || text == format!("{var} = {var} - 1") {
        return true;
    }

    // Try tree-sitter: update_expression (i++)
    if node.kind() == "update_expression" {
        let arg = node.child_by_field_name("argument")
            .map(|n| node_text(n, source));
        if arg == Some(var) {
            return true;
        }
    }

    true // be permissive for now
}

pub(crate) fn find_function<'a>(root: Node<'a>, source: &str, name: &str) -> Option<Node<'a>> {
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        if child.kind() == "function_definition" {
            if let Some(declarator) = child.child_by_field_name("declarator") {
                let text = node_text(declarator, source);
                if text.contains(name) {
                    return Some(child);
                }
            }
        }
    }
    None
}

pub(crate) fn node_text<'a>(node: Node, source: &'a str) -> &'a str {
    &source[node.byte_range()]
}

pub(crate) fn infer_width_from_type(node: Node, source: &str) -> u32 {
    let text = node_text(node, source);
    if text.contains("ulong") || text.contains("long") || text.contains("uint64") {
        64
    } else if text.contains("uint") || text.contains("int") || text.contains("uint32") {
        32
    } else if text.contains("ushort") || text.contains("short") || text.contains("uint16") {
        16
    } else if text.contains("uchar") || text.contains("char") || text.contains("uint8") {
        8
    } else {
        32
    }
}

pub(crate) fn is_signed_type(node: Node, source: &str) -> bool {
    let text = node_text(node, source);
    // Signed if: int/long/short/char without unsigned/u prefix
    if text.contains("uint") || text.contains("ulong") || text.contains("ushort")
        || text.contains("uchar") || text.contains("unsigned") {
        return false;
    }
    text.contains("int") || text.contains("long") || text.contains("short")
        || text.contains("char") || text.contains("sint")
}

pub(crate) fn const_width(val: u64, ctx_width: u32) -> u32 {
    if ctx_width > 0 {
        ctx_width
    } else if val <= 0xFF {
        8
    } else if val <= 0xFFFF {
        16
    } else if val <= 0xFFFF_FFFF {
        32
    } else {
        64
    }
}

pub(crate) fn parse_number(text: &str) -> u64 {
    let text = text.trim_end_matches(|c: char| c == 'u' || c == 'U' || c == 'l' || c == 'L');
    if let Some(hex) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        u64::from_str_radix(hex, 16).unwrap_or(0)
    } else if text.starts_with('0') && text.len() > 1 && !text.contains('.') {
        u64::from_str_radix(&text[1..], 8).unwrap_or(0)
    } else {
        text.parse().unwrap_or(0)
    }
}

pub(crate) fn op_latency(op: &DfgOp) -> u32 {
    match op {
        DfgOp::Mul => 3,
        DfgOp::Div | DfgOp::Mod => 6,
        DfgOp::Load { .. } | DfgOp::Store { .. } => 2,
        DfgOp::Const(_) | DfgOp::Input(_) | DfgOp::Output(_) | DfgOp::Nop => 0,
        DfgOp::And | DfgOp::Or | DfgOp::Xor | DfgOp::Not => 0,
        DfgOp::Eq | DfgOp::Ne | DfgOp::Lt | DfgOp::Gt | DfgOp::Le | DfgOp::Ge => 0,
        DfgOp::Mux => 0,
        _ => 1,
    }
}

use tree_sitter::{Node, Tree};

use crate::standard::{Violation, is_banned_function, is_banned_include};

pub fn validate_source(tree: &Tree, source: &str) -> Vec<Located<Violation>> {
    let mut violations = Vec::new();
    let root = tree.root_node();
    walk_node(root, source, &mut violations);
    violations
}

#[derive(Debug)]
pub struct Located<T> {
    pub line: usize,
    pub col: usize,
    pub inner: T,
}

fn walk_node(node: Node, source: &str, violations: &mut Vec<Located<Violation>>) {
    match node.kind() {
        "preproc_include" => check_include(node, source, violations),
        "call_expression" => check_call(node, source, violations),
        "goto_statement" => {
            violations.push(located(node, Violation::Goto));
        }
        "pointer_declarator" => check_pointer_arithmetic(node, source, violations),
        "declaration" => check_declaration(node, source, violations),
        "for_statement" => check_loop_bound(node, source, violations),
        "while_statement" => check_while_bound(node, source, violations),
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_node(child, source, violations);
    }
}

fn check_include(node: Node, source: &str, violations: &mut Vec<Located<Violation>>) {
    let text = node_text(node, source);
    if is_banned_include(&text) {
        violations.push(located(node, Violation::StdlibInclude));
    }
}

fn check_call(node: Node, source: &str, violations: &mut Vec<Located<Violation>>) {
    if let Some(func) = node.child_by_field_name("function") {
        let name = node_text(func, source);
        if is_banned_function(&name) {
            violations.push(located(node, Violation::DynamicAllocation));
        }
        if name == "va_start" || name == "va_end" || name == "va_arg" {
            violations.push(located(node, Violation::Varargs));
        }
    }
}

fn check_pointer_arithmetic(node: Node, source: &str, violations: &mut Vec<Located<Violation>>) {
    let parent = node.parent();
    if let Some(p) = parent {
        if p.kind() == "binary_expression" {
            let op = p.child_by_field_name("operator").map(|n| node_text(n, source));
            if matches!(op.as_deref(), Some("+") | Some("-")) {
                violations.push(located(node, Violation::PointerArithmetic));
            }
        }
    }
}

fn check_declaration(node: Node, source: &str, violations: &mut Vec<Located<Violation>>) {
    let text = node_text(node, source);
    if text.contains("float ") || text.contains("double ") {
        violations.push(located(node, Violation::FloatingPoint));
    }
}

fn check_loop_bound(node: Node, source: &str, violations: &mut Vec<Located<Violation>>) {
    if let Some(cond) = node.child_by_field_name("condition") {
        let text = node_text(cond, source);
        if text.contains("()") && !text.contains("fd_") {
            violations.push(located(
                node,
                Violation::UnboundedLoop(text.to_string()),
            ));
        }
    }
}

fn check_while_bound(node: Node, source: &str, violations: &mut Vec<Located<Violation>>) {
    if let Some(cond) = node.child_by_field_name("condition") {
        let text = node_text(cond, source);
        if text == "1" || text == "true" {
            violations.push(located(
                node,
                Violation::UnboundedLoop("while(1)".to_string()),
            ));
        }
    }
}

fn node_text<'a>(node: Node, source: &'a str) -> &'a str {
    &source[node.byte_range()]
}

fn located(node: Node, inner: Violation) -> Located<Violation> {
    let point = node.start_position();
    Located {
        line: point.row + 1,
        col: point.column + 1,
        inner,
    }
}

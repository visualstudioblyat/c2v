mod helpers;

use std::collections::HashMap;

use tree_sitter::{Node, Tree};

use super::{ArrayDecl, Dfg, DfgEdge, DfgNode, DfgOp, Direction, EdgeId, NodeId, Port, StructDef, StructField};
use helpers::*;

pub fn build_dfg(tree: &Tree, source: &str, name: &str) -> Result<Dfg, DfgBuildError> {
    let root = tree.root_node();

    let mut builder = DfgBuilder::new(name.to_string(), root);

    /* First pass: collect type definitions and function bodies */
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        match child.kind() {
            "struct_specifier" | "type_definition" => {
                builder.collect_struct_def(child, source);
            }
            "enum_specifier" => {
                builder.collect_enum_def(child, source);
            }
            "function_definition" => {
                builder.collect_function(child, source);
            }
            _ => {}
        }
    }

    let func =
        find_function(root, source, name).ok_or(DfgBuildError::FunctionNotFound(name.to_string()))?;
    builder.build_function(func, source)?;
    Ok(builder.finish())
}

#[derive(Debug, thiserror::Error)]
pub enum DfgBuildError {
    #[error("function not found: {0}")]
    FunctionNotFound(String),
    #[error("unsupported construct at line {0}: {1}")]
    Unsupported(usize, String),
}

struct ArrayInfo {
    elem_width: u32,
    size: usize,
}

#[derive(Clone)]
struct StructInfo {
    fields: Vec<(String, u32)>,
    total_width: u32,
}

#[derive(Clone)]
struct FuncInfo {
    param_names: Vec<String>,
    param_widths: Vec<u32>,
    return_width: u32,
    body_start: usize,
    body_end: usize,
}

struct DfgBuilder<'a> {
    name: String,
    root: Node<'a>,
    nodes: Vec<DfgNode>,
    edges: Vec<DfgEdge>,
    inputs: Vec<Port>,
    outputs: Vec<Port>,
    next_node: NodeId,
    next_edge: EdgeId,
    vars: HashMap<String, NodeId>,
    arrays: HashMap<String, ArrayInfo>,
    struct_defs: HashMap<String, StructInfo>,
    enum_values: HashMap<String, u64>,
    var_types: HashMap<String, String>,
    return_width: u32,
    known_funcs: HashMap<String, FuncInfo>,
    inline_depth: u32,
    inline_return_id: Option<NodeId>,
}

impl<'a> DfgBuilder<'a> {
    fn new(name: String, root: Node<'a>) -> Self {
        Self {
            name,
            root,
            nodes: Vec::new(),
            edges: Vec::new(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            next_node: 0,
            next_edge: 0,
            vars: HashMap::new(),
            arrays: HashMap::new(),
            struct_defs: HashMap::new(),
            enum_values: HashMap::new(),
            var_types: HashMap::new(),
            return_width: 32,
            known_funcs: HashMap::new(),
            inline_depth: 0,
            inline_return_id: None,
        }
    }

    /* collect_enum_def parses an enum definition and registers each
       enumerator as a named constant. Handles both:
       `enum { A, B, C }` (auto-numbered 0,1,2) and
       `enum { A=5, B=10 }` (explicit values). */
    fn collect_enum_def(&mut self, node: Node, source: &str) {
        let mut cursor = node.walk();
        let mut next_val = 0u64;

        for child in node.children(&mut cursor) {
            if child.kind() == "enumerator_list" {
                let mut ec = child.walk();
                for enumerator in child.children(&mut ec) {
                    if enumerator.kind() == "enumerator" {
                        let mut name = String::new();
                        let mut explicit_val = None;

                        let mut ic = enumerator.walk();
                        for inner in enumerator.children(&mut ic) {
                            if inner.kind() == "identifier" {
                                name = node_text(inner, source).to_string();
                            }
                            if inner.kind() == "number_literal" {
                                explicit_val = Some(parse_number(node_text(inner, source)));
                            }
                        }

                        let val = explicit_val.unwrap_or(next_val);
                        if !name.is_empty() {
                            self.enum_values.insert(name, val);
                        }
                        next_val = val + 1;
                    }
                }
            }
        }
    }

    fn collect_function(&mut self, node: Node, source: &str) {
        let ret_width = infer_width_from_type(node, source);
        if let Some(declarator) = node.child_by_field_name("declarator") {
            let decl_text = node_text(declarator, source);
            let func_name = decl_text.split('(').next().unwrap_or("").trim().to_string();

            let mut param_names = Vec::new();
            let mut param_widths = Vec::new();

            let mut cursor = declarator.walk();
            for child in declarator.children(&mut cursor) {
                if child.kind() == "parameter_list" {
                    let mut pc = child.walk();
                    for param in child.children(&mut pc) {
                        if param.kind() == "parameter_declaration" {
                            let w = infer_width_from_type(param, source);
                            let pname = param.child_by_field_name("declarator")
                                .map(|n| node_text(n, source).to_string())
                                .unwrap_or_default();
                            param_names.push(pname);
                            param_widths.push(w);
                        }
                    }
                }
            }

            if let Some(body) = node.child_by_field_name("body") {
                let range = body.byte_range();
                self.known_funcs.insert(func_name, FuncInfo {
                    param_names,
                    param_widths,
                    return_width: ret_width,
                    body_start: range.start,
                    body_end: range.end,
                });
            }
        }
    }

    fn collect_struct_def(&mut self, node: Node, source: &str) {
        let _text = node_text(node, source);

        // Handle: typedef struct { ... } name_t;
        // Handle: struct name { ... };
        let mut struct_name = None;
        let mut fields = Vec::new();

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "struct_specifier" || child.kind() == "type_definition" {
                self.collect_struct_def(child, source);
                return;
            }
            if child.kind() == "type_identifier" {
                struct_name = Some(node_text(child, source).to_string());
            }
            if child.kind() == "field_declaration_list" {
                let mut fc = child.walk();
                for field in child.children(&mut fc) {
                    if field.kind() == "field_declaration" {
                        let width = infer_width_from_type(field, source);
                        if let Some(decl) = field.child_by_field_name("declarator") {
                            let fname = node_text(decl, source).to_string();
                            fields.push((fname, width));
                        }
                    }
                }
            }
        }

        // For typedef: the last type_identifier is the typedef name
        if struct_name.is_none() {
            let mut cursor2 = node.walk();
            let children: Vec<_> = node.children(&mut cursor2).collect();
            for child in children.iter().rev() {
                if child.kind() == "type_identifier" {
                    struct_name = Some(node_text(*child, source).to_string());
                    break;
                }
            }
        }

        if let Some(name) = struct_name {
            if !fields.is_empty() {
                let _offset = 0u32;
                let total: u32 = fields.iter().map(|(_, w)| w).sum();
                self.struct_defs.insert(name, StructInfo { fields, total_width: total });
            }
        }
    }

    fn add_node(&mut self, op: DfgOp, width: u32, line: usize, name: &str) -> NodeId {
        let id = self.next_node;
        self.next_node += 1;
        let latency = op_latency(&op);
        self.nodes.push(DfgNode {
            id,
            op,
            width,
            signed: false,
            latency,
            source_line: line,
            source_name: name.to_string(),
        });
        id
    }

    fn add_node_signed(&mut self, op: DfgOp, width: u32, signed: bool, line: usize, name: &str) -> NodeId {
        let id = self.next_node;
        self.next_node += 1;
        let latency = op_latency(&op);
        self.nodes.push(DfgNode {
            id,
            op,
            width,
            signed,
            latency,
            source_line: line,
            source_name: name.to_string(),
        });
        id
    }

    fn add_edge(&mut self, from: NodeId, to: NodeId, width: u32) -> EdgeId {
        let id = self.next_edge;
        self.next_edge += 1;
        self.edges.push(DfgEdge { id, from, to, width });
        id
    }

    fn build_function(&mut self, func: Node, source: &str) -> Result<(), DfgBuildError> {
        self.return_width = infer_width_from_type(func, source);

        if let Some(declarator) = func.child_by_field_name("declarator") {
            self.extract_params(declarator, source);
        }

        if let Some(body) = func.child_by_field_name("body") {
            self.build_body(body, source)?;
        }

        Ok(())
    }

    /* extract_params walks the function's parameter list and creates
       input ports + DFG Input nodes. Handles three cases:
       - Scalar: `uint x` → one input port
       - Pointer-to-struct: `state_t *ctx` → expand struct fields as ports
       - Array: `uint data[N]` → treated as N scalar inputs (future) */
    fn extract_params(&mut self, declarator: Node, source: &str) {
        let mut cursor = declarator.walk();
        for child in declarator.children(&mut cursor) {
            if child.kind() == "parameter_list" {
                let mut param_cursor = child.walk();
                for param in child.children(&mut param_cursor) {
                    if param.kind() != "parameter_declaration" { continue; }

                    let decl = match param.child_by_field_name("declarator") {
                        Some(d) => d,
                        None    => continue,
                    };

                    let line = param.start_position().row + 1;

                    /* Pointer-to-struct: `type_t *name` */
                    if decl.kind() == "pointer_declarator" {
                        self.extract_pointer_param(param, decl, source, line);
                        continue;
                    }

                    /* Scalar parameter */
                    let name   = node_text(decl, source).to_string();
                    let width  = infer_width_from_type(param, source);
                    let signed = is_signed_type(param, source);

                    self.inputs.push(Port {
                        name: name.clone(),
                        width,
                        direction: Direction::In,
                    });
                    let id = self.add_node_signed(
                        DfgOp::Input(name.clone()),
                        width, signed, line, &name,
                    );
                    self.vars.insert(name, id);
                }
            }
        }
    }

    /* extract_pointer_param handles `type_t *name` parameters by
       expanding the struct's fields into individual input ports.
       For `state_t *ctx` with fields {x: u32, y: u32}, this creates
       ports `ctx_x` and `ctx_y`, and maps `ctx->x` to the `ctx_x` input.
       If the pointee type is not a known struct, treats it as a single
       scalar input (pointer-to-scalar). */
    fn extract_pointer_param(
        &mut self,
        param: Node,
        decl:  Node,
        source: &str,
        line:  usize,
    ) {
        /* Extract the variable name from inside the pointer_declarator.
           pointer_declarator has children: "*", then the identifier */
        let mut inner_name = String::new();
        let mut dc = decl.walk();
        for child in decl.children(&mut dc) {
            if child.kind() == "identifier" {
                inner_name = node_text(child, source).to_string();
            }
        }
        if inner_name.is_empty() { return; }

        /* Resolve the struct type from the parameter's type specifier */
        let type_text = node_text(param, source);
        let struct_type = self.struct_defs.keys()
            .find(|k| type_text.contains(k.as_str()))
            .cloned();

        if let Some(ref st_name) = struct_type {
            if let Some(sdef) = self.struct_defs.get(st_name).cloned() {
                /* Register this variable as a struct type */
                self.var_types.insert(inner_name.clone(), st_name.clone());

                /* Create one input port per struct field */
                for (fname, fwidth) in &sdef.fields {
                    let port_name = format!("{}_{}", inner_name, fname);
                    self.inputs.push(Port {
                        name:      port_name.clone(),
                        width:     *fwidth,
                        direction: Direction::In,
                    });
                    let id = self.add_node(
                        DfgOp::Input(port_name.clone()),
                        *fwidth, line, &port_name,
                    );
                    /* Map both `ctx.field` and `ctx_field` to this input */
                    let qualified = format!("{}.{}", inner_name, fname);
                    self.vars.insert(qualified, id);
                    self.vars.insert(port_name, id);
                }
                return;
            }
        }

        /* Fallback: pointer-to-scalar — treat as single input */
        let width  = infer_width_from_type(param, source);
        let signed = is_signed_type(param, source);
        self.inputs.push(Port {
            name:      inner_name.clone(),
            width,
            direction: Direction::In,
        });
        let id = self.add_node_signed(
            DfgOp::Input(inner_name.clone()),
            width, signed, line, &inner_name,
        );
        self.vars.insert(inner_name, id);
    }

    fn build_body(&mut self, body: Node, source: &str) -> Result<(), DfgBuildError> {
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            self.build_statement(child, source)?;
        }
        Ok(())
    }

    fn build_statement(&mut self, node: Node, source: &str) -> Result<(), DfgBuildError> {
        match node.kind() {
            "declaration" => self.build_declaration(node, source),
            "expression_statement" => self.build_expression_statement(node, source),
            "return_statement" => self.build_return(node, source),
            "for_statement" => self.build_for_loop(node, source),
            "compound_statement" => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    self.build_statement(child, source)?;
                }
                Ok(())
            }
            "if_statement" => self.build_if(node, source),
            "while_statement" => self.build_while(node, source),
            "switch_statement" => self.build_switch(node, source),
            "labeled_statement" => {
                // case X: stmt — just build the inner statement
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() != "case" && child.kind() != "default"
                        && child.kind() != ":" && !child.kind().contains("literal") {
                        self.build_statement(child, source)?;
                    }
                }
                Ok(())
            }
            "break_statement" => Ok(()), // breaks handled by switch builder
            _ => Ok(()),
        }
    }

    fn build_for_loop(&mut self, node: Node, source: &str) -> Result<(), DfgBuildError> {
        let line = node.start_position().row + 1;

        let init = node.child_by_field_name("initializer");
        let cond = node.child_by_field_name("condition");
        let update = node.child_by_field_name("update");
        let body = node.child_by_field_name("body");

        let (var_name, start_val, end_val) = extract_loop_bounds(init, cond, update, source)
            .ok_or(DfgBuildError::Unsupported(line, "non-constant loop bounds — unroll manually".to_string()))?;

        let iter_count = (start_val - end_val).unsigned_abs().max((end_val - start_val).unsigned_abs());
        if iter_count > 1024 {
            return Err(DfgBuildError::Unsupported(line, format!("loop too large to unroll: {iter_count} iterations")));
        }

        let body = body.ok_or(DfgBuildError::Unsupported(line, "empty for body".to_string()))?;

        let step: i64 = if start_val <= end_val { 1 } else { -1 };
        let mut i = start_val;
        loop {
            if step > 0 && i >= end_val { break; }
            if step < 0 && i < end_val { break; }
            let const_id = self.add_node(
                DfgOp::Const(i as u64), 32, line,
                &format!("c_{i}"),
            );
            self.vars.insert(var_name.clone(), const_id);
            self.build_statement(body, source)?;
            i += step;
        }

        Ok(())
    }

    fn build_if(&mut self, node: Node, source: &str) -> Result<(), DfgBuildError> {
        let line = node.start_position().row + 1;
        let cond_node = node.child_by_field_name("condition");
        let consequence = node.child_by_field_name("consequence");
        let alternative = node.child_by_field_name("alternative");

        let cond_node = match cond_node {
            Some(c) => c,
            None => return Ok(()),
        };

        // Build condition expression
        // condition is wrapped in parenthesized_expression
        let cond_inner = if cond_node.kind() == "parenthesized_expression" {
            let mut cursor = cond_node.walk();
            cond_node.children(&mut cursor)
                .find(|c| c.kind() != "(" && c.kind() != ")")
                .unwrap_or(cond_node)
        } else {
            cond_node
        };
        let cond_id = self.build_expr(cond_inner, source, 1)?;

        // Snapshot vars before if
        let vars_before = self.vars.clone();

        // Build true branch
        let mut vars_true = vars_before.clone();
        if let Some(cons) = consequence {
            self.vars = vars_before.clone();
            self.build_statement(cons, source)?;
            vars_true = self.vars.clone();
        }

        // Build false branch
        let mut vars_false = vars_before.clone();
        if let Some(alt) = alternative {
            self.vars = vars_before.clone();
            // alternative node may be wrapped: else { ... } or else if { ... }
            let alt_body = if alt.kind() == "else_clause" {
                let mut cursor = alt.walk();
                alt.children(&mut cursor).find(|c| c.kind() != "else").unwrap_or(alt)
            } else {
                alt
            };
            self.build_statement(alt_body, source)?;
            vars_false = self.vars.clone();
        }

        // For each variable that differs between branches, insert a mux
        let mut merged = vars_before.clone();
        for (var, &true_id) in &vars_true {
            let false_id = vars_false.get(var).copied()
                .or_else(|| vars_before.get(var).copied());
            let before_id = vars_before.get(var).copied();

            match (before_id, false_id) {
                (Some(before), Some(false_val)) if true_id != before || false_val != before => {
                    let true_val = true_id;
                    let false_val = if alternative.is_some() { false_val } else { before };
                    if true_val != false_val {
                        let width = self.nodes[true_val as usize].width
                            .max(self.nodes[false_val as usize].width);
                        let mux_id = self.add_node(DfgOp::Mux, width, line, "mux");
                        self.add_edge(cond_id, mux_id, 1);
                        self.add_edge(true_val, mux_id, width);
                        self.add_edge(false_val, mux_id, width);
                        merged.insert(var.clone(), mux_id);
                    } else {
                        merged.insert(var.clone(), true_val);
                    }
                }
                _ => {
                    merged.insert(var.clone(), true_id);
                }
            }
        }

        // Also check vars that only changed in false branch
        for (var, &false_id) in &vars_false {
            if !vars_true.contains_key(var) {
                if let Some(&before) = vars_before.get(var) {
                    if false_id != before {
                        let width = self.nodes[false_id as usize].width
                            .max(self.nodes[before as usize].width);
                        let mux_id = self.add_node(DfgOp::Mux, width, line, "mux");
                        self.add_edge(cond_id, mux_id, 1);
                        self.add_edge(before, mux_id, width);
                        self.add_edge(false_id, mux_id, width);
                        merged.insert(var.clone(), mux_id);
                    }
                }
            }
        }

        self.vars = merged;
        Ok(())
    }

    fn build_while(&mut self, node: Node, _source: &str) -> Result<(), DfgBuildError> {
        let line = node.start_position().row + 1;
        Err(DfgBuildError::Unsupported(
            line,
            "while loops cannot be synthesized — use for(int i=0; i<N; i++) with constant bounds".to_string(),
        ))
    }

    fn build_switch(&mut self, node: Node, source: &str) -> Result<(), DfgBuildError> {
        let line = node.start_position().row + 1;

        // switch(expr) { case A: ...; break; case B: ...; break; default: ...; }
        // → chain of if/else: if(expr==A) {...} else if(expr==B) {...} else {...}
        let cond_node = node.child_by_field_name("condition");
        let body_node = node.child_by_field_name("body");

        let cond_node = match cond_node {
            Some(c) => c,
            None => return Err(DfgBuildError::Unsupported(line, "switch without condition".to_string())),
        };

        // Build the switch expression
        let cond_inner = if cond_node.kind() == "parenthesized_expression" {
            let mut cursor = cond_node.walk();
            cond_node.children(&mut cursor)
                .find(|c| c.kind() != "(" && c.kind() != ")")
                .unwrap_or(cond_node)
        } else {
            cond_node
        };
        let switch_val = self.build_expr(cond_inner, source, 32)?;

        let body = match body_node {
            Some(b) => b,
            None => return Ok(()),
        };

        // Collect cases: (value_or_default, statements)
        struct CaseArm<'a> {
            value: Option<i64>,
            stmts: Vec<tree_sitter::Node<'a>>,
        }

        let mut arms: Vec<CaseArm> = Vec::new();
        let mut current_arm: Option<CaseArm> = None;

        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            match child.kind() {
                "case_statement" => {
                    if let Some(arm) = current_arm.take() {
                        arms.push(arm);
                    }
                    // Extract case value
                    let mut sc = child.walk();
                    let mut case_val = None;
                    let mut stmts = Vec::new();
                    for sc_child in child.children(&mut sc) {
                        if sc_child.kind() == "number_literal" {
                            case_val = Some(parse_number(node_text(sc_child, source)) as i64);
                        } else if sc_child.kind() != "case" && sc_child.kind() != ":"
                            && sc_child.kind() != "break_statement" {
                            stmts.push(sc_child);
                        }
                    }
                    current_arm = Some(CaseArm { value: case_val, stmts });
                }
                "default_statement" => {
                    if let Some(arm) = current_arm.take() {
                        arms.push(arm);
                    }
                    let mut sc = child.walk();
                    let mut stmts = Vec::new();
                    for sc_child in child.children(&mut sc) {
                        if sc_child.kind() != "default" && sc_child.kind() != ":"
                            && sc_child.kind() != "break_statement" {
                            stmts.push(sc_child);
                        }
                    }
                    current_arm = Some(CaseArm { value: None, stmts });
                }
                _ => {
                    if let Some(ref mut arm) = current_arm {
                        if child.kind() != "break_statement" && child.kind() != "{" && child.kind() != "}" {
                            arm.stmts.push(child);
                        }
                    }
                }
            }
        }
        if let Some(arm) = current_arm.take() {
            arms.push(arm);
        }

        // Build as cascaded mux: for each case, snapshot vars, build body,
        // then merge with condition check
        let vars_before = self.vars.clone();
        let mut result_vars = vars_before.clone();

        for arm in &arms {
            let vars_snapshot = result_vars.clone();
            self.vars = vars_snapshot.clone();

            // Build this arm's statements
            for stmt in &arm.stmts {
                self.build_statement(*stmt, source)?;
            }
            let vars_after = self.vars.clone();

            if let Some(case_val) = arm.value {
                // case VALUE: — insert mux for changed vars
                let width = self.nodes[switch_val as usize].width;
                let const_id = self.add_node(
                    DfgOp::Const(case_val as u64), width, line,
                    &format!("c_{case_val}"),
                );
                let eq_id = self.add_node(DfgOp::Eq, 1, line, "eq");
                self.add_edge(switch_val, eq_id, width);
                self.add_edge(const_id, eq_id, width);

                for (var, &new_id) in &vars_after {
                    let old_id = vars_snapshot.get(var).copied().unwrap_or(new_id);
                    if new_id != old_id {
                        let w = self.nodes[new_id as usize].width
                            .max(self.nodes[old_id as usize].width);
                        let mux_id = self.add_node(DfgOp::Mux, w, line, "mux");
                        self.add_edge(eq_id, mux_id, 1);
                        self.add_edge(new_id, mux_id, w);
                        self.add_edge(old_id, mux_id, w);
                        result_vars.insert(var.clone(), mux_id);
                    }
                }
            } else {
                // default: — just use the values directly
                for (var, &new_id) in &vars_after {
                    result_vars.insert(var.clone(), new_id);
                }
            }
        }

        self.vars = result_vars;
        Ok(())
    }

    fn build_declaration(&mut self, node: Node, source: &str) -> Result<(), DfgBuildError> {
        let width = infer_width_from_type(node, source);

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "init_declarator" {
                let decl = child.child_by_field_name("declarator");
                let value = child.child_by_field_name("value");

                if let Some(decl) = decl {
                    let decl_text = node_text(decl, source);

                    // Check if this is an array declaration: name[SIZE]
                    if decl.kind() == "array_declarator" {
                        let arr_name = decl.child_by_field_name("declarator")
                            .map(|n| node_text(n, source).to_string())
                            .unwrap_or_default();
                        let arr_size = decl.child_by_field_name("size")
                            .and_then(|n| node_text(n, source).parse::<usize>().ok())
                            .unwrap_or(0);

                        self.arrays.insert(arr_name.clone(), ArrayInfo {
                            elem_width: width,
                            size: arr_size,
                        });

                        // If there's an initializer list, emit stores
                        if let Some(value) = value {
                            if value.kind() == "initializer_list" {
                                let mut idx = 0usize;
                                let mut val_cursor = value.walk();
                                for item in value.children(&mut val_cursor) {
                                    if item.kind() == "{" || item.kind() == "}" || item.kind() == "," {
                                        continue;
                                    }
                                    let line = item.start_position().row + 1;
                                    let val_id = self.build_expr(item, source, width)?;
                                    let idx_id = self.add_node(
                                        DfgOp::Const(idx as u64), 32, line,
                                        &format!("c_{idx}"),
                                    );
                                    let store_id = self.add_node(
                                        DfgOp::Store { array: arr_name.clone() },
                                        width, line,
                                        &format!("st_{arr_name}"),
                                    );
                                    self.add_edge(idx_id, store_id, 32);
                                    self.add_edge(val_id, store_id, width);
                                    idx += 1;
                                }
                            }
                        }
                    } else if let Some(value) = value {
                        let var_name = decl_text.to_string();
                        // Check if this is a struct type
                        let type_text = node_text(node, source);
                        let struct_type = self.struct_defs.keys()
                            .find(|k| type_text.contains(k.as_str()))
                            .cloned();
                        if let Some(ref st) = struct_type {
                            self.var_types.insert(var_name.clone(), st.clone());
                        }

                        if value.kind() == "initializer_list" {
                            // Struct initializer: type var = { .field = val, ... } or { val, val, ... }
                            if let Some(ref st) = struct_type {
                                if let Some(sdef) = self.struct_defs.get(st).cloned() {
                                    let mut field_idx = 0;
                                    let mut val_cursor = value.walk();
                                    for item in value.children(&mut val_cursor) {
                                        if item.kind() == "{" || item.kind() == "}" || item.kind() == "," { continue; }
                                        if field_idx < sdef.fields.len() {
                                            let (fname, fwidth) = &sdef.fields[field_idx];
                                            let val_id = self.build_expr(item, source, *fwidth)?;
                                            let qualified = format!("{var_name}.{fname}");
                                            self.vars.insert(qualified, val_id);
                                            field_idx += 1;
                                        }
                                    }
                                }
                            }
                        } else {
                            let val_id = self.build_expr(value, source, width)?;
                            self.vars.insert(var_name, val_id);
                        }
                    } else {
                        // Declaration without initializer — might be a struct type
                        let var_name = decl_text.to_string();
                        let type_text = node_text(node, source);
                        let struct_type = self.struct_defs.keys()
                            .find(|k| type_text.contains(k.as_str()))
                            .cloned();
                        if let Some(st) = struct_type {
                            self.var_types.insert(var_name, st);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn build_expression_statement(
        &mut self,
        node: Node,
        source: &str,
    ) -> Result<(), DfgBuildError> {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "assignment_expression" => { self.build_assignment(child, source)?; }
                "augmented_assignment_expression" => { self.build_augmented_assignment(child, source)?; }
                "update_expression" => { /* i++ / i-- — handled in loop unroll */ }
                _ => {}
            }
        }
        Ok(())
    }

    fn build_augmented_assignment(
        &mut self,
        node: Node,
        source: &str,
    ) -> Result<(), DfgBuildError> {
        let line = node.start_position().row + 1;
        let left = node.child_by_field_name("left").unwrap();
        let right = node.child_by_field_name("right").unwrap();
        let op_node = node.child_by_field_name("operator").unwrap();
        let op_text = node_text(op_node, source);

        let var_name = node_text(left, source).to_string();
        let width = self.vars.get(&var_name)
            .map(|&id| self.nodes[id as usize].width)
            .unwrap_or(32);

        let old_id = self.vars.get(&var_name).copied()
            .unwrap_or_else(|| self.add_node(DfgOp::Const(0), width, line, "zero"));
        let rhs_id = self.build_expr(right, source, width)?;

        let (op, name) = match op_text {
            "+=" => (DfgOp::Add, "add"),
            "-=" => (DfgOp::Sub, "sub"),
            "*=" => (DfgOp::Mul, "mul"),
            "&=" => (DfgOp::And, "band"),
            "|=" => (DfgOp::Or, "bor"),
            "^=" => (DfgOp::Xor, "bxor"),
            "<<=" => (DfgOp::Shl, "shl"),
            ">>=" => (DfgOp::Shr, "shr"),
            _ => return Err(DfgBuildError::Unsupported(line, format!("operator: {op_text}"))),
        };

        let result_id = self.add_node(op, width, line, name);
        self.add_edge(old_id, result_id, width);
        self.add_edge(rhs_id, result_id, width);
        self.vars.insert(var_name, result_id);
        Ok(())
    }

    fn build_assignment(&mut self, node: Node, source: &str) -> Result<(), DfgBuildError> {
        let left = node.child_by_field_name("left");
        let right = node.child_by_field_name("right");

        if let (Some(left), Some(right)) = (left, right) {
            let line = node.start_position().row + 1;

            if left.kind() == "subscript_expression" {
                // arr[idx] = val  →  Store node
                let arr_node = left.child_by_field_name("argument").unwrap();
                let idx_node = left.child_by_field_name("index").unwrap();
                let arr_name = node_text(arr_node, source).to_string();
                let elem_width = self.arrays.get(&arr_name)
                    .map(|a| a.elem_width).unwrap_or(32);

                let idx_id = self.build_expr(idx_node, source, 32)?;
                let val_id = self.build_expr(right, source, elem_width)?;
                let store_id = self.add_node(
                    DfgOp::Store { array: arr_name.clone() },
                    elem_width, line,
                    &format!("st_{arr_name}"),
                );
                self.add_edge(idx_id, store_id, 32);
                self.add_edge(val_id, store_id, elem_width);
            } else if left.kind() == "field_expression" {
                // obj.field = val  or  ptr->field = val
                let arg = left.child_by_field_name("argument").unwrap();
                let field = left.child_by_field_name("field").unwrap();
                let var_name = node_text(arg, source).to_string();
                let field_name = node_text(field, source).to_string();

                let struct_type = self.var_types.get(&var_name).cloned();
                let field_width = struct_type.as_ref()
                    .and_then(|st| self.struct_defs.get(st))
                    .and_then(|sd| sd.fields.iter().find(|(n, _)| n == &field_name))
                    .map(|(_, w)| *w)
                    .unwrap_or(32);

                let val_id = self.build_expr(right, source, field_width)?;
                let qualified = format!("{var_name}.{field_name}");
                self.vars.insert(qualified, val_id);
            } else {
                let var_name = node_text(left, source).to_string();
                let width = self.vars.get(&var_name)
                    .map(|&id| self.nodes[id as usize].width)
                    .unwrap_or(64);
                let val_id = self.build_expr(right, source, width)?;
                self.vars.insert(var_name, val_id);
            }
        }
        Ok(())
    }

    fn build_return(&mut self, node: Node, source: &str) -> Result<(), DfgBuildError> {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() != "return" && child.kind() != ";" {
                let ret_id = self.build_expr(child, source, self.return_width)?;

                if self.inline_depth > 0 {
                    // Inside inlined function — capture return value, don't create Output
                    self.inline_return_id = Some(ret_id);
                } else {
                    // Top-level function — create module output
                    let width = self.nodes[ret_id as usize].width;
                    if self.outputs.is_empty() {
                        let out_id = self.add_node(
                            DfgOp::Output("result".to_string()),
                            width,
                            node.start_position().row + 1,
                            "result",
                        );
                        self.add_edge(ret_id, out_id, width);
                        self.outputs.push(Port {
                            name: "result".to_string(),
                            width,
                            direction: Direction::Out,
                        });
                    }
                }
                break;
            }
        }
        Ok(())
    }

    fn build_expr(
        &mut self,
        node: Node,
        source: &str,
        ctx_width: u32,
    ) -> Result<NodeId, DfgBuildError> {
        let line = node.start_position().row + 1;

        match node.kind() {
            "binary_expression" => {
                let left = node.child_by_field_name("left").unwrap();
                let right = node.child_by_field_name("right").unwrap();
                let op_node = node.child_by_field_name("operator").unwrap();
                let op_text = node_text(op_node, source);

                let is_comparison = matches!(op_text,
                    "==" | "!=" | "<" | ">" | "<=" | ">=" | "&&" | "||");

                let operand_ctx = if is_comparison { 32 } else { ctx_width };
                let left_id = self.build_expr(left, source, operand_ctx)?;
                let right_id = self.build_expr(right, source, operand_ctx)?;

                let operand_width = self.nodes[left_id as usize]
                    .width
                    .max(self.nodes[right_id as usize].width);

                let (op, op_name) = match op_text {
                    "+" => (DfgOp::Add, "add"),
                    "-" => (DfgOp::Sub, "sub"),
                    "*" => (DfgOp::Mul, "mul"),
                    "/" => (DfgOp::Div, "div"),
                    "%" => (DfgOp::Mod, "mod"),
                    "<<" => (DfgOp::Shl, "shl"),
                    ">>" => (DfgOp::Shr, "shr"),
                    "&" => (DfgOp::And, "and"),
                    "|" => (DfgOp::Or, "or"),
                    "^" => (DfgOp::Xor, "xor"),
                    "==" => (DfgOp::Eq, "eq"),
                    "!=" => (DfgOp::Ne, "ne"),
                    "<" => (DfgOp::Lt, "lt"),
                    ">" => (DfgOp::Gt, "gt"),
                    "<=" => (DfgOp::Le, "le"),
                    ">=" => (DfgOp::Ge, "ge"),
                    "&&" => (DfgOp::And, "land"),
                    "||" => (DfgOp::Or, "lor"),
                    _ => {
                        return Err(DfgBuildError::Unsupported(
                            line,
                            format!("operator: {op_text}"),
                        ))
                    }
                };

                let result_width = if is_comparison { 1 } else { operand_width };
                let signed = self.nodes[left_id as usize].signed
                    || self.nodes[right_id as usize].signed;
                let result_id = self.add_node_signed(op, result_width, signed, line, op_name);
                self.add_edge(left_id, result_id, operand_width);
                self.add_edge(right_id, result_id, operand_width);
                Ok(result_id)
            }
            "unary_expression" => {
                let op_node = node.child_by_field_name("operator").unwrap();
                let operand = node.child_by_field_name("argument").unwrap();
                let op_text = node_text(op_node, source);

                let operand_id = self.build_expr(operand, source, ctx_width)?;
                let width = self.nodes[operand_id as usize].width;

                match op_text {
                    "~" => {
                        let result_id = self.add_node(DfgOp::Not, width, line, "not");
                        self.add_edge(operand_id, result_id, width);
                        Ok(result_id)
                    }
                    "-" => {
                        /* Unary minus: -x = 0 - x */
                        let zero_id = self.add_node(DfgOp::Const(0), width, line, "c_0");
                        let result_id = self.add_node_signed(DfgOp::Sub, width, true, line, "neg");
                        self.add_edge(zero_id, result_id, width);
                        self.add_edge(operand_id, result_id, width);
                        Ok(result_id)
                    }
                    "!" => {
                        /* Logical NOT: !x = (x == 0) */
                        let zero_id = self.add_node(DfgOp::Const(0), width, line, "c_0");
                        let result_id = self.add_node(DfgOp::Eq, 1, line, "lnot");
                        self.add_edge(operand_id, result_id, width);
                        self.add_edge(zero_id, result_id, width);
                        Ok(result_id)
                    }
                    _ => Err(DfgBuildError::Unsupported(
                        line,
                        format!("unary operator: {op_text}"),
                    )),
                }
            }
            "number_literal" => {
                let text = node_text(node, source);
                let val = parse_number(text);
                let width = const_width(val, ctx_width);
                Ok(self.add_node(DfgOp::Const(val), width, line, &format!("c_{val}")))
            }
            "identifier" => {
                let name = node_text(node, source);
                /* Check variables first, then enum constants, then treat as input */
                if let Some(&id) = self.vars.get(name) {
                    Ok(id)
                } else if let Some(&val) = self.enum_values.get(name) {
                    Ok(self.add_node(DfgOp::Const(val), ctx_width, line, name))
                } else {
                    let id = self.add_node(DfgOp::Input(name.to_string()), ctx_width, line, name);
                    self.vars.insert(name.to_string(), id);
                    Ok(id)
                }
            }
            "parenthesized_expression" => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() != "(" && child.kind() != ")" {
                        return self.build_expr(child, source, ctx_width);
                    }
                }
                Err(DfgBuildError::Unsupported(line, "empty parens".to_string()))
            }
            "cast_expression" => {
                if let Some(value) = node.child_by_field_name("value") {
                    self.build_expr(value, source, ctx_width)
                } else {
                    let mut cursor = node.walk();
                    let children: Vec<_> = node.children(&mut cursor).collect();
                    if let Some(last) = children.last() {
                        self.build_expr(*last, source, ctx_width)
                    } else {
                        Err(DfgBuildError::Unsupported(line, "empty cast".to_string()))
                    }
                }
            }
            "field_expression" => {
                // obj.field or ptr->field
                let arg = node.child_by_field_name("argument").unwrap();
                let field = node.child_by_field_name("field").unwrap();
                let field_name = node_text(field, source).to_string();
                let var_name = node_text(arg, source).to_string();

                // Look up the struct type for this variable
                let struct_type = self.var_types.get(&var_name).cloned();
                let field_width = struct_type.as_ref()
                    .and_then(|st| self.struct_defs.get(st))
                    .and_then(|sd| sd.fields.iter().find(|(n, _)| n == &field_name))
                    .map(|(_, w)| *w)
                    .unwrap_or(ctx_width);

                // Each struct.field is tracked as a separate variable: "var.field"
                let qualified = format!("{var_name}.{field_name}");
                if let Some(&id) = self.vars.get(&qualified) {
                    Ok(id)
                } else {
                    let id = self.add_node(
                        DfgOp::FieldGet {
                            struct_name: struct_type.unwrap_or_default(),
                            field: field_name.clone(),
                        },
                        field_width, line, &qualified,
                    );
                    self.vars.insert(qualified, id);
                    Ok(id)
                }
            }
            "subscript_expression" => {
                // arr[idx] → Load node
                let arr_node = node.child_by_field_name("argument").unwrap();
                let idx_node = node.child_by_field_name("index").unwrap();
                let arr_name = node_text(arr_node, source).to_string();
                let elem_width = self.arrays.get(&arr_name)
                    .map(|a| a.elem_width).unwrap_or(ctx_width);

                let idx_id = self.build_expr(idx_node, source, 32)?;
                let load_id = self.add_node(
                    DfgOp::Load { array: arr_name.clone() },
                    elem_width, line,
                    &format!("ld_{arr_name}"),
                );
                self.add_edge(idx_id, load_id, 32);

                // Chain load after most recent store to same array (RAW dependency)
                let last_store = self.nodes.iter().rev().find(|n| {
                    matches!(&n.op, DfgOp::Store { array } if array == &arr_name)
                }).map(|n| n.id);
                if let Some(store_id) = last_store {
                    self.add_edge(store_id, load_id, 0);
                }

                Ok(load_id)
            }
            "call_expression" => {
                let func_node = node.child_by_field_name("function");
                let args_node = node.child_by_field_name("arguments");

                if let Some(func_node) = func_node {
                    let fname = node_text(func_node, source).to_string();

                    // Check if we know this function
                    let func_info = self.known_funcs.get(&fname).cloned();
                    if let Some(info) = func_info {
                        if self.inline_depth > 16 {
                            return Err(DfgBuildError::Unsupported(line, format!("recursion depth exceeded inlining {fname}")));
                        }

                        // Build argument expressions
                        let mut arg_ids = Vec::new();
                        if let Some(args) = args_node {
                            let mut cursor = args.walk();
                            for child in args.children(&mut cursor) {
                                if child.kind() == "(" || child.kind() == ")" || child.kind() == "," {
                                    continue;
                                }
                                let arg_id = self.build_expr(child, source, ctx_width)?;
                                arg_ids.push(arg_id);
                            }
                        }

                        // Save caller's vars, bind params to args
                        let saved_vars = self.vars.clone();
                        let saved_return_width = self.return_width;
                        self.return_width = info.return_width;
                        self.inline_depth += 1;

                        for (i, pname) in info.param_names.iter().enumerate() {
                            if let Some(&arg_id) = arg_ids.get(i) {
                                self.vars.insert(pname.clone(), arg_id);
                            }
                        }

                        // Find the callee in the tree and inline its body
                        let callee = find_function(self.root, source, &fname);
                        self.inline_return_id = None;

                        if let Some(callee) = callee {
                            if let Some(body) = callee.child_by_field_name("body") {
                                self.build_body(body, source)?;
                            }
                        }

                        let return_id = self.inline_return_id;

                        // Restore caller's vars (keep new nodes/edges)
                        self.inline_depth -= 1;
                        self.return_width = saved_return_width;
                        self.inline_return_id = None;
                        self.vars = saved_vars;

                        match return_id {
                            Some(id) => Ok(id),
                            None => Err(DfgBuildError::Unsupported(line, format!("{fname} has no return value")))
                        }
                    } else {
                        Err(DfgBuildError::Unsupported(
                            line,
                            format!("unknown function: {fname}"),
                        ))
                    }
                } else {
                    Err(DfgBuildError::Unsupported(line, "indirect call".to_string()))
                }
            }
            "conditional_expression" => {
                let cond = node.child_by_field_name("condition").unwrap();
                let cons = node.child_by_field_name("consequence").unwrap();
                let alt = node.child_by_field_name("alternative").unwrap();

                let cond_id = self.build_expr(cond, source, 1)?;
                let cons_id = self.build_expr(cons, source, ctx_width)?;
                let alt_id = self.build_expr(alt, source, ctx_width)?;

                let width = self.nodes[cons_id as usize]
                    .width
                    .max(self.nodes[alt_id as usize].width);
                let mux_id = self.add_node(DfgOp::Mux, width, line, "mux");
                self.add_edge(cond_id, mux_id, 1);
                self.add_edge(cons_id, mux_id, width);
                self.add_edge(alt_id, mux_id, width);
                Ok(mux_id)
            }
            "sizeof_expression" => {
                /* sizeof(type) or sizeof(expr) — resolve to constant.
                   For our restricted subset, we know the sizes. */
                let text = node_text(node, source);
                let size = if text.contains("ulong") || text.contains("long") || text.contains("uint64") {
                    8u64
                } else if text.contains("uint") || text.contains("int") || text.contains("uint32") {
                    4
                } else if text.contains("ushort") || text.contains("short") {
                    2
                } else if text.contains("uchar") || text.contains("char") {
                    1
                } else {
                    4 /* default to 4 bytes */
                };
                Ok(self.add_node(DfgOp::Const(size), 32, line, &format!("sizeof_{size}")))
            }
            "comma_expression" => {
                /* (a, b) — evaluate both, return b */
                let mut last_id = None;
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() != "," {
                        last_id = Some(self.build_expr(child, source, ctx_width)?);
                    }
                }
                last_id.ok_or(DfgBuildError::Unsupported(line, "empty comma expression".to_string()))
            }
            _ => Err(DfgBuildError::Unsupported(
                line,
                format!("expression kind: {}", node.kind()),
            )),
        }
    }

    fn finish(self) -> Dfg {
        let arrays = self.arrays.iter().map(|(name, info)| ArrayDecl {
            name: name.clone(),
            elem_width: info.elem_width,
            size: info.size,
        }).collect();

        let structs = self.struct_defs.iter().map(|(name, info)| {
            let mut offset = 0u32;
            let fields = info.fields.iter().map(|(fname, fwidth)| {
                let f = StructField { name: fname.clone(), width: *fwidth, offset };
                offset += fwidth;
                f
            }).collect();
            StructDef { name: name.clone(), fields, total_width: info.total_width }
        }).collect();

        Dfg {
            name: self.name,
            nodes: self.nodes,
            edges: self.edges,
            inputs: self.inputs,
            outputs: self.outputs,
            arrays,
            structs,
        }
    }
}


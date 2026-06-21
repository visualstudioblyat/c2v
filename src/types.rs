use std::collections::HashMap;
use tree_sitter::{Node, Tree};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Void,
    Bool,
    U8,
    U16,
    U32,
    U64,
    U128,
    I8,
    I16,
    I32,
    I64,
    Array(Box<Type>, usize),
    Struct(String),
    Ptr(Box<Type>),
}

impl Type {
    pub fn width(&self) -> u32 {
        match self {
            Type::Void => 0,
            Type::Bool => 1,
            Type::U8 | Type::I8 => 8,
            Type::U16 | Type::I16 => 16,
            Type::U32 | Type::I32 => 32,
            Type::U64 | Type::I64 => 64,
            Type::U128 => 128,
            Type::Array(elem, _) => elem.width(),
            Type::Struct(_) => 0,
            Type::Ptr(_) => 64,
        }
    }

    pub fn is_signed(&self) -> bool {
        matches!(self, Type::I8 | Type::I16 | Type::I32 | Type::I64)
    }
}

#[derive(Debug, Clone)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<(String, Type)>,
}

impl StructDef {
    pub fn field_type(&self, name: &str) -> Option<&Type> {
        self.fields.iter().find(|(n, _)| n == name).map(|(_, t)| t)
    }

    pub fn total_width(&self) -> u32 {
        self.fields.iter().map(|(_, t)| t.width()).sum()
    }
}

#[derive(Debug, Clone)]
pub struct FuncSig {
    pub name: String,
    pub params: Vec<(String, Type)>,
    pub ret: Type,
}

#[derive(Debug)]
pub struct TypeTable {
    pub typedefs: HashMap<String, Type>,
    pub structs: HashMap<String, StructDef>,
    pub functions: HashMap<String, FuncSig>,
    pub locals: HashMap<String, Type>,
}

impl TypeTable {
    pub fn new() -> Self {
        let mut typedefs = HashMap::new();
        typedefs.insert("uchar".into(), Type::U8);
        typedefs.insert("ushort".into(), Type::U16);
        typedefs.insert("uint".into(), Type::U32);
        typedefs.insert("ulong".into(), Type::U64);
        typedefs.insert("uint128".into(), Type::U128);
        typedefs.insert("schar".into(), Type::I8);

        Self {
            typedefs,
            structs: HashMap::new(),
            functions: HashMap::new(),
            locals: HashMap::new(),
        }
    }

    pub fn resolve(&self, name: &str) -> Type {
        if let Some(t) = self.typedefs.get(name) {
            return t.clone();
        }
        if self.structs.contains_key(name) {
            return Type::Struct(name.to_string());
        }
        match name {
            "void" => Type::Void,
            "char" | "signed char" | "schar" => Type::I8,
            "unsigned char" | "uchar" => Type::U8,
            "short" | "signed short" => Type::I16,
            "unsigned short" | "ushort" => Type::U16,
            "int" | "signed int" | "signed" => Type::I32,
            "unsigned int" | "unsigned" | "uint" => Type::U32,
            "long" | "signed long" | "long int" => Type::I64,
            "unsigned long" | "ulong" | "unsigned long int" => Type::U64,
            _ => Type::U32,
        }
    }

    pub fn resolve_node(&self, node: Node, source: &str) -> Type {
        let text = node_text(node, source).trim();

        // Check for struct type
        if text.contains("struct ") {
            let struct_name = text.split_whitespace()
                .skip_while(|w| *w != "struct")
                .nth(1)
                .unwrap_or("");
            if self.structs.contains_key(struct_name) {
                return Type::Struct(struct_name.to_string());
            }
        }

        // Check if it's a known typedef (including struct typedefs)
        let type_words: Vec<&str> = text.split_whitespace()
            .take_while(|w| !w.contains('(') && !w.contains('[') && !w.contains('=') && !w.contains(';'))
            .filter(|w| *w != "const" && *w != "static" && *w != "inline" && *w != "volatile"
                && *w != "register" && *w != "extern" && *w != "typedef")
            .collect();

        // Try each word as a typedef
        for word in type_words.iter().rev() {
            if let Some(t) = self.typedefs.get(*word) {
                return t.clone();
            }
            if self.structs.contains_key(*word) {
                return Type::Struct(word.to_string());
            }
        }

        // Fall back to keyword matching
        let joined = type_words.join(" ");
        self.resolve(&joined)
    }

    pub fn width_of(&self, ty: &Type) -> u32 {
        match ty {
            Type::Struct(name) => {
                self.structs.get(name).map(|s| s.total_width()).unwrap_or(0)
            }
            other => other.width(),
        }
    }

    pub fn collect_from_tree(&mut self, tree: &Tree, source: &str) {
        let root = tree.root_node();
        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            match child.kind() {
                "type_definition" => self.collect_typedef(child, source),
                "struct_specifier" => self.collect_struct(child, source),
                "declaration" => self.collect_global_decl(child, source),
                "function_definition" => self.collect_func_sig(child, source),
                _ => {}
            }
        }
    }

    fn collect_typedef(&mut self, node: Node, source: &str) {
        let mut cursor = node.walk();
        let children: Vec<_> = node.children(&mut cursor).collect();

        // typedef <type_spec> <name>;
        // typedef struct { ... } <name>;
        let mut has_struct_body = false;
        let mut struct_fields = Vec::new();
        let mut typedef_name = None;

        for child in &children {
            if child.kind() == "struct_specifier" {
                has_struct_body = true;
                let mut sc = child.walk();
                for sn in child.children(&mut sc) {
                    if sn.kind() == "field_declaration_list" {
                        struct_fields = self.parse_field_list(sn, source);
                    }
                }
            }
            if child.kind() == "type_identifier" {
                typedef_name = Some(node_text(*child, source).to_string());
            }
        }

        if let Some(name) = typedef_name {
            if has_struct_body && !struct_fields.is_empty() {
                self.structs.insert(name.clone(), StructDef {
                    name: name.clone(),
                    fields: struct_fields,
                });
                self.typedefs.insert(name.clone(), Type::Struct(name));
            } else {
                // Simple typedef: typedef unsigned int uint;
                let base_type = self.resolve_node(node, source);
                if base_type != Type::Struct(name.clone()) {
                    self.typedefs.insert(name, base_type);
                }
            }
        }
    }

    fn collect_struct(&mut self, node: Node, source: &str) {
        let mut name = None;
        let mut fields = Vec::new();

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "type_identifier" {
                name = Some(node_text(child, source).to_string());
            }
            if child.kind() == "field_declaration_list" {
                fields = self.parse_field_list(child, source);
            }
        }

        if let Some(name) = name {
            if !fields.is_empty() {
                self.structs.insert(name.clone(), StructDef { name, fields });
            }
        }
    }

    fn parse_field_list(&self, node: Node, source: &str) -> Vec<(String, Type)> {
        let mut fields = Vec::new();
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "field_declaration" {
                let ty = self.resolve_node(child, source);
                if let Some(decl) = child.child_by_field_name("declarator") {
                    let name = node_text(decl, source).to_string();
                    fields.push((name, ty));
                }
            }
        }
        fields
    }

    fn collect_global_decl(&mut self, _node: Node, _source: &str) {
        // For future: global constants
    }

    fn collect_func_sig(&mut self, node: Node, source: &str) {
        let ret_type = self.resolve_node(node, source);
        if let Some(declarator) = node.child_by_field_name("declarator") {
            let func_text = node_text(declarator, source);
            let func_name = func_text.split('(').next().unwrap_or("").trim().to_string();

            let mut params = Vec::new();
            let mut cursor = declarator.walk();
            for child in declarator.children(&mut cursor) {
                if child.kind() == "parameter_list" {
                    let mut pc = child.walk();
                    for param in child.children(&mut pc) {
                        if param.kind() == "parameter_declaration" {
                            let pty = self.resolve_node(param, source);
                            let pname = param.child_by_field_name("declarator")
                                .map(|n| node_text(n, source).to_string())
                                .unwrap_or_default();
                            params.push((pname, pty));
                        }
                    }
                }
            }

            self.functions.insert(func_name.clone(), FuncSig {
                name: func_name,
                params,
                ret: ret_type,
            });
        }
    }
}

fn node_text<'a>(node: Node, source: &'a str) -> &'a str {
    &source[node.byte_range()]
}

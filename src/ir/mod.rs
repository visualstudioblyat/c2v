mod build;
pub mod dot;
pub mod entropy;
pub mod transform;
pub mod tree_balance;
pub mod validate;

pub use build::build_dfg;

use serde::Serialize;
use std::collections::HashMap;

pub type NodeId = u32;
pub type EdgeId = u32;

#[derive(Debug, Clone, Serialize)]
pub struct ArrayDecl {
    pub name: String,
    pub elem_width: u32,
    pub size: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<StructField>,
    pub total_width: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct StructField {
    pub name: String,
    pub width: u32,
    pub offset: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct Dfg {
    pub name: String,
    pub nodes: Vec<DfgNode>,
    pub edges: Vec<DfgEdge>,
    pub inputs: Vec<Port>,
    pub outputs: Vec<Port>,
    pub arrays: Vec<ArrayDecl>,
    pub structs: Vec<StructDef>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DfgNode {
    pub id: NodeId,
    pub op: DfgOp,
    pub width: u32,
    pub signed: bool,
    pub latency: u32,
    pub source_line: usize,
    pub source_name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DfgEdge {
    pub id: EdgeId,
    pub from: NodeId,
    pub to: NodeId,
    pub width: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct Port {
    pub name: String,
    pub width: u32,
    pub direction: Direction,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub enum Direction {
    In,
    Out,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum DfgOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Shl,
    Shr,
    And,
    Or,
    Xor,
    Not,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    Mux,
    Reg,
    Load { array: String },
    Store { array: String },
    FieldGet { struct_name: String, field: String },
    FieldSet { struct_name: String, field: String },
    Const(u64),
    Input(String),
    Output(String),
    Nop,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScheduledDfg {
    pub dfg: Dfg,
    pub schedule: HashMap<NodeId, u32>,
    pub total_cycles: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct BoundDfg {
    pub scheduled: ScheduledDfg,
    pub bindings: HashMap<NodeId, ResourceId>,
    pub resources: Vec<Resource>,
}

pub type ResourceId = u32;

#[derive(Debug, Clone, Serialize)]
pub struct Resource {
    pub id: ResourceId,
    pub kind: ResourceKind,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum ResourceKind {
    Adder { width: u32 },
    Multiplier { width: u32 },
    Shifter { width: u32 },
    Comparator { width: u32 },
    Mux { width: u32, inputs: u32 },
    Bram { depth: u32, width: u32 },
    Register { width: u32 },
}

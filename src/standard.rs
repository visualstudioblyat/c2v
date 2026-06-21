use thiserror::Error;

#[derive(Debug, Error)]
pub enum Violation {
    #[error("malloc/free/calloc/realloc forbidden — use fd_scratch or static allocation")]
    DynamicAllocation,
    #[error("recursion forbidden — all call graphs must be acyclic")]
    Recursion,
    #[error("function pointer forbidden — use tile dispatch table")]
    FunctionPointer,
    #[error("goto forbidden — use structured control flow")]
    Goto,
    #[error("global mutable state forbidden — use workspace or tile context")]
    GlobalMutable,
    #[error("varargs forbidden — use fixed-arity functions")]
    Varargs,
    #[error("pointer arithmetic forbidden — use fd_chunk_to_laddr / typed array indexing")]
    PointerArithmetic,
    #[error("floating point forbidden — use fixed-point or integer")]
    FloatingPoint,
    #[error("VLA forbidden — all array sizes must be compile-time constants")]
    VariableLengthArray,
    #[error("cast to non-POD type forbidden — only POD struct casts allowed")]
    NonPodCast,
    #[error("stdlib include forbidden — use fd_util_base primitives")]
    StdlibInclude,
    #[error("missing lifecycle function: {0} has no _{1}")]
    MissingLifecycle(String, String),
    #[error("naming violation: {0} does not follow fd_<component>_<verb> pattern")]
    NamingViolation(String),
    #[error("unbounded loop: {0} has no computable iteration count")]
    UnboundedLoop(String),
    #[error("dynamic array size: {0} — size must be a constant expression")]
    DynamicArraySize(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Construct {
    Lifecycle,
    Tile,
    Channel,
    Scratch,
    Simd,
    Template,
    PureFunction,
}

#[derive(Debug, Clone)]
pub struct AcceptedFunction {
    pub name: String,
    pub construct: Construct,
    pub params: Vec<Param>,
    pub return_type: CType,
    pub body_ops: Vec<Op>,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: CType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CType {
    Void,
    Uchar,
    Ushort,
    Uint,
    Ulong,
    Schar,
    Short,
    Int,
    Long,
    Uint128,
    Array(Box<CType>, usize),
    Ptr(Box<CType>),
    Struct(String),
}

#[derive(Debug, Clone)]
pub enum Op {
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
    Load,
    Store,
    Call(String),
    BoundedLoop { bound: usize },
    Branch,
    Return,
    ScratchAlloc,
    ChannelRead,
    ChannelWrite,
    MemFence,
}

pub const BANNED_FUNCTIONS: &[&str] = &[
    "malloc", "calloc", "realloc", "free",
    "printf", "fprintf", "sprintf", "snprintf",
    "scanf", "fscanf", "sscanf",
    "fopen", "fclose", "fread", "fwrite",
    "exit", "abort", "atexit",
    "setjmp", "longjmp",
    "signal", "raise",
    "system", "popen",
];

pub const BANNED_INCLUDES: &[&str] = &[
    "stdio.h", "stdlib.h", "string.h", "math.h",
    "signal.h", "setjmp.h", "stdarg.h",
    "unistd.h", "pthread.h", "dlfcn.h",
];

pub const REQUIRED_LIFECYCLE: &[&str] = &[
    "align", "footprint", "new", "join", "leave", "delete",
];

pub fn is_fd_naming(name: &str) -> bool {
    if name.starts_with("fd_") {
        let rest = &name[3..];
        let parts: Vec<&str> = rest.splitn(3, '_').collect();
        parts.len() >= 2
    } else {
        false
    }
}

pub fn is_banned_function(name: &str) -> bool {
    BANNED_FUNCTIONS.contains(&name)
}

pub fn is_banned_include(path: &str) -> bool {
    BANNED_INCLUDES.iter().any(|b| path.contains(b))
}

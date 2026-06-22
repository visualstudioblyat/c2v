use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    #[serde(default)]
    pub schedule_overrides: HashMap<String, u32>,
    #[serde(default)]
    pub bind_overrides: HashMap<String, String>,
    #[serde(default)]
    pub resource_limits: ResourceLimits,
    #[serde(default)]
    pub options: PolicyOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    #[serde(default = "default_max_add")]
    pub max_adders: u32,
    #[serde(default = "default_max_mul")]
    pub max_multipliers: u32,
    #[serde(default = "default_max_shift")]
    pub max_shifters: u32,
    #[serde(default = "default_max_cmp")]
    pub max_comparators: u32,
}

fn default_max_add() -> u32 { 8 }
fn default_max_mul() -> u32 { 4 }
fn default_max_shift() -> u32 { 8 }
fn default_max_cmp() -> u32 { 4 }

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_adders: 8,
            max_multipliers: 4,
            max_shifters: 8,
            max_comparators: 4,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyOptions {
    #[serde(default = "default_true")]
    pub tree_balance: bool,
    #[serde(default = "default_true")]
    pub strength_reduce: bool,
    #[serde(default = "default_true")]
    pub const_fold: bool,
    #[serde(default = "default_true")]
    pub cse: bool,
    #[serde(default)]
    pub pipeline: bool,
    #[serde(default)]
    pub target: Option<String>,
}

fn default_true() -> bool { true }

impl Default for PolicyOptions {
    fn default() -> Self {
        Self {
            tree_balance: true,
            strength_reduce: true,
            const_fold: true,
            cse: true,
            pipeline: false,
            target: None,
        }
    }
}

impl Policy {
    pub fn load(path: &Path) -> Result<Self, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("cannot read policy file: {e}"))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("invalid policy JSON: {e}"))
    }

    pub fn empty() -> Self {
        Self {
            schedule_overrides: HashMap::new(),
            bind_overrides: HashMap::new(),
            resource_limits: ResourceLimits::default(),
            options: PolicyOptions::default(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct BatchResult {
    pub function: String,
    pub policy_id: String,
    pub cycles: u32,
    pub luts: u32,
    pub ffs: u32,
    pub dsps: u32,
    pub brams: u32,
    pub verified: bool,
    pub trace_file: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BatchConfig {
    pub source_files: Vec<String>,
    pub function: String,
    pub policies: Vec<PolicyVariant>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PolicyVariant {
    pub id: String,
    pub policy: Policy,
}

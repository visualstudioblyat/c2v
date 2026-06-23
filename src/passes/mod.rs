pub mod bit_narrow;
pub mod chain_combine;
pub mod constant_prop;
pub mod dead_node;
pub mod const_fold;
pub mod cse;
pub mod duplicate_store;
pub mod identity_elim;
pub mod loop_invariant;
pub mod mux_simplify;
pub mod mux_to_case;
pub mod reassociate;
pub mod strength_reduce;
pub mod width_narrow;

use crate::ir::Dfg;

pub type PassResult = Result<Dfg, PassError>;

#[derive(Debug, thiserror::Error)]
pub enum PassError {
    #[error("pass {0} failed: {1}")]
    Failed(String, String),
}

pub struct PassPipeline {
    passes: Vec<Box<dyn Pass>>,
}

pub trait Pass {
    fn name(&self) -> &'static str;
    fn run(&self, dfg: Dfg) -> PassResult;
}

impl PassPipeline {
    pub fn new() -> Self {
        Self { passes: Vec::new() }
    }

    pub fn add(&mut self, pass: Box<dyn Pass>) {
        self.passes.push(pass);
    }

    pub fn run(&self, mut dfg: Dfg) -> PassResult {
        for pass in &self.passes {
            let t0 = std::time::Instant::now();
            let n_before = dfg.nodes.len();
            dfg = pass.run(dfg)?;
            let n_after = dfg.nodes.len();
            let elapsed = t0.elapsed();
            if n_before != n_after {
                eprintln!("[pass]      {:<20} {} → {} ops{:>8}",
                    pass.name(), n_before, n_after, format!("✓  {:?}", elapsed));
            }
        }
        Ok(dfg)
    }
}

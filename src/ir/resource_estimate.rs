/* c2v_resource_estimate computes pre-synthesis area estimates:
   LUT count, register count, DSP slice usage, BRAM usage, and
   estimated maximum frequency. These are rough estimates based
   on operation counts and don't replace actual synthesis — but
   they give immediate feedback during compilation, before the
   20-minute Vivado run.

   Estimation model:
     - Each arithmetic op → ~32 LUTs (adder chain)
     - Each multiply → 1 DSP48E1 (or ~100 LUTs if no DSP)
     - Each register → width flip-flops
     - Each array → ceil(size * width / 36Kbit) BRAMs
     - Each mux → ~width LUTs (MUX tree)
     - Each compare → ~width/4 LUTs */

use crate::ir::{BoundDfg, DfgOp};
use crate::target::TargetSpec;

#[derive(Debug, Clone)]
pub struct ResourceEstimate {
    pub luts:       u32,
    pub registers:  u32,
    pub dsps:       u32,
    pub brams:      u32,
    pub fmax_mhz:   f64,
    pub detail:     ResourceDetail,
}

#[derive(Debug, Clone)]
pub struct ResourceDetail {
    pub arith_luts:  u32,
    pub mux_luts:    u32,
    pub logic_luts:  u32,
    pub ctrl_luts:   u32,
    pub data_regs:   u32,
    pub ctrl_regs:   u32,
}

pub fn estimate_resources(
    bound:  &BoundDfg,
    target: &TargetSpec,
) -> ResourceEstimate {
    let dfg = &bound.scheduled.dfg;

    let mut arith_luts = 0u32;
    let mut mux_luts   = 0u32;
    let mut logic_luts = 0u32;
    let mut ctrl_luts  = 0u32;
    let mut data_regs  = 0u32;
    let mut dsps       = 0u32;

    for node in &dfg.nodes {
        let w = node.width;
        match &node.op {
            DfgOp::Input(_) | DfgOp::Const(_) | DfgOp::Output(_) | DfgOp::Nop => {}

            DfgOp::Add | DfgOp::Sub => {
                arith_luts += w;
                data_regs  += w;
            }

            DfgOp::Mul => {
                if target.dsp_slices > 0 && w <= 25 {
                    dsps += 1;
                } else if target.dsp_slices > 0 && w <= 48 {
                    dsps += 2;
                } else {
                    arith_luts += w * w / 4;
                }
                data_regs += w;
            }

            DfgOp::Div | DfgOp::Mod => {
                arith_luts += w * 3;
                data_regs  += w;
            }

            DfgOp::Shl | DfgOp::Shr => {
                /* Barrel shifter */
                let stages = (w as f64).log2().ceil() as u32;
                logic_luts += w * stages;
                data_regs  += w;
            }

            DfgOp::And | DfgOp::Or | DfgOp::Xor | DfgOp::Not => {
                logic_luts += (w + 3) / 4;
                data_regs  += w;
            }

            DfgOp::Eq | DfgOp::Ne | DfgOp::Lt | DfgOp::Gt |
            DfgOp::Le | DfgOp::Ge => {
                arith_luts += (w + 3) / 4;
                data_regs  += 1;
            }

            DfgOp::Mux => {
                mux_luts  += w;
                data_regs += w;
            }

            DfgOp::Reg => {
                data_regs += w;
            }

            DfgOp::Load { .. } => {
                data_regs += w;
            }

            DfgOp::Store { .. } => {}

            DfgOp::FieldGet { .. } | DfgOp::FieldSet { .. } => {
                data_regs += w;
            }
        }
    }

    /* FSM control registers */
    let state_bits = crate::emit::state_width( bound.scheduled.total_cycles );
    let ctrl_regs  = state_bits + 2; /* state + done + start */
    ctrl_luts += bound.scheduled.total_cycles * 2;

    /* BRAM estimation */
    let mut brams = 0u32;
    for arr in &dfg.arrays {
        let total_bits = arr.size as u64 * arr.elem_width as u64;
        let bram_bits  = target.bram_bits as u64;
        if bram_bits > 0 {
            brams += ((total_bits + bram_bits - 1) / bram_bits) as u32;
        }
    }

    let total_luts = arith_luts + mux_luts + logic_luts + ctrl_luts;
    let total_regs = data_regs + ctrl_regs;

    /* Fmax estimation from critical path and target speed grade */
    let critical_ns = estimate_critical_path_ns( bound, target );
    let fmax_mhz = if critical_ns > 0.0 { 1000.0 / critical_ns } else { target.max_freq_mhz };

    ResourceEstimate {
        luts:      total_luts,
        registers: total_regs,
        dsps,
        brams,
        fmax_mhz,
        detail: ResourceDetail {
            arith_luts,
            mux_luts,
            logic_luts,
            ctrl_luts,
            data_regs,
            ctrl_regs,
        },
    }
}

fn estimate_critical_path_ns(
    bound:  &BoundDfg,
    target: &TargetSpec,
) -> f64 {
    let dfg = &bound.scheduled.dfg;

    /* Find the cycle with the most operations and estimate
       the combinational delay of chained operations. */
    let mut max_delay = 0.0f64;

    for cycle in 0..bound.scheduled.total_cycles {
        let mut cycle_delay = 0.0f64;

        for node in &dfg.nodes {
            if bound.scheduled.schedule.get( &node.id ) != Some( &cycle ) {
                continue;
            }
            let op_delay = match &node.op {
                DfgOp::Add | DfgOp::Sub   => target.add_delay_ns,
                DfgOp::Mul                 => target.mul_delay_ns,
                DfgOp::Div | DfgOp::Mod   => target.mul_delay_ns * 3.0,
                DfgOp::Shl | DfgOp::Shr   => target.add_delay_ns * 0.8,
                DfgOp::And | DfgOp::Or |
                DfgOp::Xor | DfgOp::Not   => target.add_delay_ns * 0.3,
                DfgOp::Eq  | DfgOp::Ne |
                DfgOp::Lt  | DfgOp::Gt |
                DfgOp::Le  | DfgOp::Ge    => target.add_delay_ns * 0.8,
                DfgOp::Mux                 => target.mux_delay_ns,
                _ => 0.0,
            };
            cycle_delay = cycle_delay.max( op_delay );
        }

        /* Add routing delay estimate */
        cycle_delay += target.route_delay_ns;
        max_delay = max_delay.max( cycle_delay );
    }

    max_delay
}

impl std::fmt::Display for ResourceEstimate {
    fn fmt( &self, f: &mut std::fmt::Formatter<'_> ) -> std::fmt::Result {
        writeln!( f, "Resource Estimate:" )?;
        writeln!( f, "  LUTs:      {:>6}  (arith: {}, mux: {}, logic: {}, ctrl: {})",
            self.luts, self.detail.arith_luts, self.detail.mux_luts,
            self.detail.logic_luts, self.detail.ctrl_luts )?;
        writeln!( f, "  Registers: {:>6}  (data: {}, ctrl: {})",
            self.registers, self.detail.data_regs, self.detail.ctrl_regs )?;
        writeln!( f, "  DSPs:      {:>6}", self.dsps )?;
        writeln!( f, "  BRAMs:     {:>6}", self.brams )?;
        writeln!( f, "  Fmax:      {:>5.1} MHz", self.fmax_mhz )?;
        Ok(())
    }
}

#![allow(non_camel_case_types)]
use crate::ir::{BoundDfg, DfgOp, ResourceKind};
use super::{TargetModel, TargetReport, TargetUtilization};

macro_rules! xilinx_target {
    ($name:ident, $display:expr, $luts:expr, $ffs:expr, $dsps:expr,
     $bram18:expr, $freq:expr, $dsp_lat:expr, $bram_lat:expr) => {
        pub struct $name;

        impl TargetModel for $name {
            fn name(&self) -> &str { $display }
            fn max_luts(&self) -> u32 { $luts }
            fn max_ffs(&self) -> u32 { $ffs }
            fn max_dsps(&self) -> u32 { $dsps }
            fn max_bram18(&self) -> u32 { $bram18 }
            fn dsp_latency(&self) -> u32 { $dsp_lat }
            fn bram_latency(&self) -> u32 { $bram_lat }
            fn max_freq_mhz(&self) -> f64 { $freq }

            fn estimate(&self, bound: &BoundDfg) -> TargetReport {
                estimate_xilinx(self, bound)
            }
        }
    };
}

xilinx_target!(Artix7_35T,            "xc7a35t",  20800,  41600,   90,  100, 250.0, 3, 2);
xilinx_target!(Artix7_100T,           "xc7a100t", 63400, 126800,  240,  270, 250.0, 3, 2);
xilinx_target!(Kintex7_325T,          "xc7k325t",203800, 407600,  840,  890, 500.0, 3, 2);
xilinx_target!(UltraScalePlus_KU040,  "xcku040", 242400, 484800, 1920, 1200, 775.0, 4, 2);
xilinx_target!(UltraScalePlus_VU9P,   "xcvu9p", 1182240,2364480, 6840, 4320, 775.0, 4, 2);

fn estimate_xilinx(target: &dyn TargetModel, bound: &BoundDfg) -> TargetReport {
    let dfg = &bound.scheduled.dfg;
    let mut luts = 0u32;
    let mut ffs = 0u32;
    let mut dsps = 0u32;
    let mut bram18 = 0u32;
    let mut warnings = Vec::new();

    // Resource-based estimation
    for r in &bound.resources {
        match &r.kind {
            ResourceKind::Adder { width } => {
                luts += width; // 1 LUT per bit for add/sub
                ffs += width;  // output register
            }
            ResourceKind::Multiplier { width } => {
                if *width <= 18 {
                    dsps += 1; // fits in one DSP48
                } else if *width <= 27 {
                    dsps += 1; // DSP48E2 handles 27x18
                } else {
                    dsps += 2; // need cascaded DSPs
                }
                ffs += width * 2;
            }
            ResourceKind::Shifter { width } => {
                luts += width * 2; // barrel shifter
                ffs += width;
            }
            ResourceKind::Comparator { width } => {
                luts += width / 2 + 1; // carry chain comparator
            }
            ResourceKind::Mux { width, inputs } => {
                // Xilinx 6-LUT can implement 3:1 mux per LUT
                let mux_luts = ((*inputs + 2) / 3) * width;
                luts += mux_luts;
            }
            ResourceKind::Bram { depth, width } => {
                let bits = depth * width;
                if bits <= 18432 { bram18 += 1; }
                else { bram18 += (bits + 18431) / 18432; }
            }
            ResourceKind::Register { width } => {
                ffs += width;
            }
        }
    }

    // Array-based BRAM estimation
    for arr in &dfg.arrays {
        let bits = (arr.elem_width as usize * arr.size) as u32;
        if bits > 256 {
            let needed = (bits + 18431) / 18432;
            bram18 += needed;
        } else {
            ffs += bits; // distributed RAM
        }
    }

    // FSM state register
    let state_bits = ((bound.scheduled.total_cycles + 2) as f64).log2().ceil() as u32;
    ffs += state_bits;
    luts += state_bits * 2; // next-state logic

    // Mux nodes in the DFG (from if/else)
    for node in &dfg.nodes {
        if matches!(&node.op, DfgOp::Mux) {
            luts += node.width; // each mux bit needs a LUT
            ffs += node.width;
        }
    }

    // Pipeline registers for each intermediate node
    for node in &dfg.nodes {
        match &node.op {
            DfgOp::Input(_) | DfgOp::Const(_) | DfgOp::Output(_) | DfgOp::Nop => {}
            _ => { ffs += node.width; }
        }
    }

    // Frequency estimation based on critical path depth
    let crit_path_ns = bound.scheduled.total_cycles as f64 * 1.0; // rough: 1ns per stage
    let achievable_freq = target.max_freq_mhz().min(1000.0 / (crit_path_ns / bound.scheduled.total_cycles as f64).max(1.0));

    // Utilization warnings
    if luts > target.max_luts() {
        warnings.push(format!("LUT usage {} exceeds target capacity {}", luts, target.max_luts()));
    }
    if dsps > target.max_dsps() {
        warnings.push(format!("DSP usage {} exceeds target capacity {}", dsps, target.max_dsps()));
    }
    if bram18 > target.max_bram18() {
        warnings.push(format!("BRAM18 usage {} exceeds target capacity {}", bram18, target.max_bram18()));
    }

    let util = TargetUtilization {
        lut_pct: luts as f64 / target.max_luts() as f64 * 100.0,
        ff_pct: ffs as f64 / target.max_ffs() as f64 * 100.0,
        dsp_pct: if target.max_dsps() > 0 { dsps as f64 / target.max_dsps() as f64 * 100.0 } else { 0.0 },
        bram_pct: if target.max_bram18() > 0 { bram18 as f64 / target.max_bram18() as f64 * 100.0 } else { 0.0 },
    };

    TargetReport {
        target_name: target.name().to_string(),
        luts, ffs, dsps, bram18,
        bram36: bram18 / 2,
        frequency_est_mhz: achievable_freq,
        utilization: util,
        warnings,
    }
}

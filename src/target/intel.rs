use crate::ir::{BoundDfg, DfgOp, ResourceKind};
use super::{TargetModel, TargetReport, TargetUtilization};

macro_rules! intel_target {
    ($name:ident, $display:expr, $alms:expr, $ffs:expr, $dsps:expr,
     $m20k:expr, $freq:expr, $dsp_lat:expr, $bram_lat:expr) => {
        pub struct $name;

        impl TargetModel for $name {
            fn name(&self) -> &str { $display }
            fn max_luts(&self) -> u32 { $alms * 2 } // ALM ≈ 2 LUTs
            fn max_ffs(&self) -> u32 { $ffs }
            fn max_dsps(&self) -> u32 { $dsps }
            fn max_bram18(&self) -> u32 { $m20k } // M20K ≈ BRAM18
            fn dsp_latency(&self) -> u32 { $dsp_lat }
            fn bram_latency(&self) -> u32 { $bram_lat }
            fn max_freq_mhz(&self) -> f64 { $freq }

            fn estimate(&self, bound: &BoundDfg) -> TargetReport {
                estimate_intel(self, bound)
            }
        }
    };
}

intel_target!(CycloneV_5CEFA5,  "5cefa5",   77000, 154000,  150,   397, 450.0, 3, 2);
intel_target!(Arria10_115,      "10ax115",  427200, 854400, 1518,  2713, 750.0, 3, 2);
intel_target!(Agilex_FB014,     "agfb014",  487200, 974400, 3960,  4510, 1000.0, 4, 2);

fn estimate_intel(target: &dyn TargetModel, bound: &BoundDfg) -> TargetReport {
    let dfg = &bound.scheduled.dfg;
    let mut alms = 0u32; // Intel uses ALMs, 1 ALM ≈ 2 LUTs
    let mut ffs = 0u32;
    let mut dsps = 0u32;
    let mut m20k = 0u32;
    let mut warnings = Vec::new();

    for r in &bound.resources {
        match &r.kind {
            ResourceKind::Adder { width } => {
                alms += (width + 1) / 2; // ALM handles 2-bit add
                ffs += width;
            }
            ResourceKind::Multiplier { width } => {
                if *width <= 18 {
                    dsps += 1;
                } else if *width <= 27 {
                    dsps += 1;
                } else {
                    dsps += 2;
                }
                ffs += width * 2;
            }
            ResourceKind::Shifter { width } => {
                alms += width;
                ffs += width;
            }
            ResourceKind::Comparator { width } => {
                alms += (width + 1) / 2;
            }
            ResourceKind::Mux { width, inputs } => {
                alms += (inputs / 2 + 1) * ((width + 1) / 2);
            }
            ResourceKind::Bram { depth, width } => {
                let bits = depth * width;
                m20k += (bits + 20479) / 20480;
            }
            ResourceKind::Register { width } => {
                ffs += width;
            }
        }
    }

    for arr in &dfg.arrays {
        let bits = (arr.elem_width as usize * arr.size) as u32;
        if bits > 320 {
            m20k += (bits + 20479) / 20480;
        } else {
            ffs += bits;
        }
    }

    // Pipeline/mux/state registers
    let state_bits = ((bound.scheduled.total_cycles + 2) as f64).log2().ceil() as u32;
    ffs += state_bits;
    alms += state_bits;

    for node in &dfg.nodes {
        match &node.op {
            DfgOp::Input(_) | DfgOp::Const(_) | DfgOp::Output(_) | DfgOp::Nop => {}
            DfgOp::Mux => { alms += (node.width + 1) / 2; ffs += node.width; }
            _ => { ffs += node.width; }
        }
    }

    let luts = alms * 2;
    let achievable_freq = target.max_freq_mhz().min(1000.0);

    if luts > target.max_luts() {
        warnings.push(format!("ALM usage {} exceeds target", alms));
    }
    if dsps > target.max_dsps() {
        warnings.push(format!("DSP usage {} exceeds target {}", dsps, target.max_dsps()));
    }

    let util = TargetUtilization {
        lut_pct: luts as f64 / target.max_luts() as f64 * 100.0,
        ff_pct: ffs as f64 / target.max_ffs() as f64 * 100.0,
        dsp_pct: if target.max_dsps() > 0 { dsps as f64 / target.max_dsps() as f64 * 100.0 } else { 0.0 },
        bram_pct: if target.max_bram18() > 0 { m20k as f64 / target.max_bram18() as f64 * 100.0 } else { 0.0 },
    };

    TargetReport {
        target_name: target.name().to_string(),
        luts, ffs, dsps,
        bram18: m20k,
        bram36: m20k / 2,
        frequency_est_mhz: achievable_freq,
        utilization: util,
        warnings,
    }
}

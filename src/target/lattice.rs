/* c2v_target_lattice provides resource models for Lattice FPGA families.
   Supports iCE40 (low-cost, open-source toolchain via Yosys+nextpnr)
   and ECP5 (mid-range, also open-source). These targets are commonly
   used in open-source hardware projects and education. */

#![allow(non_camel_case_types)]
use crate::ir::{BoundDfg, DfgOp, ResourceKind};
use super::{TargetModel, TargetReport, TargetUtilization};

macro_rules! lattice_target {
    ($name:ident, $display:expr, $luts:expr, $ffs:expr, $dsps:expr,
     $bram:expr, $freq:expr, $dsp_lat:expr, $bram_lat:expr) => {
        pub struct $name;

        impl TargetModel for $name {
            fn name(&self) -> &str { $display }
            fn max_luts(&self) -> u32 { $luts }
            fn max_ffs(&self) -> u32 { $ffs }
            fn max_dsps(&self) -> u32 { $dsps }
            fn max_bram18(&self) -> u32 { $bram }
            fn dsp_latency(&self) -> u32 { $dsp_lat }
            fn bram_latency(&self) -> u32 { $bram_lat }
            fn max_freq_mhz(&self) -> f64 { $freq }

            fn estimate(&self, bound: &BoundDfg) -> TargetReport {
                estimate_lattice( self, bound )
            }
        }
    };
}

/*                       name              display      LUTs   FFs   DSPs BRAMs freq  dsp_l bram_l */
lattice_target!( ICE40_UP5K,              "ice40up5k",  5280,  5280,  8,  30, 48.0, 3, 2 );
lattice_target!( ICE40_HX8K,              "ice40hx8k",  7680,  7680,  0,  32, 100.0, 3, 2 );
lattice_target!( ECP5_25F,                "ecp5-25f", 24000, 24000, 28, 56, 400.0, 3, 2 );
lattice_target!( ECP5_85F,                "ecp5-85f", 84000, 84000, 156, 208, 400.0, 3, 2 );
lattice_target!( CrossLink_NX_17,         "lifcl-17", 17000, 17000, 56, 96, 500.0, 3, 2 );

fn estimate_lattice( target: &dyn TargetModel, bound: &BoundDfg ) -> TargetReport {
    let dfg = &bound.scheduled.dfg;
    let mut luts    = 0u32;
    let mut ffs     = 0u32;
    let mut dsps    = 0u32;
    let mut brams   = 0u32;
    let mut warnings = Vec::new();

    for r in &bound.resources {
        match &r.kind {
            ResourceKind::Adder { width }      => { luts += width; ffs += width; }
            ResourceKind::Multiplier { width }  => {
                if target.max_dsps() > 0 && *width <= 18 {
                    dsps += 1;
                } else {
                    luts += width * width; /* LUT-based multiply */
                }
                ffs += width * 2;
            }
            ResourceKind::Shifter { width }    => { luts += width * 2; ffs += width; }
            ResourceKind::Comparator { width } => { luts += width; }
            ResourceKind::Mux { width, inputs } => { luts += width * inputs; }
            ResourceKind::Bram { .. }          => { brams += 1; }
            ResourceKind::Register { width }   => { ffs += width; }
        }
    }

    for arr in &dfg.arrays {
        let bits = ( arr.elem_width as usize * arr.size ) as u32;
        if bits > 128 {
            brams += ( bits + 4095 ) / 4096; /* EBR = 4Kbit blocks */
        } else {
            ffs += bits;
        }
    }

    let state_bits = (( bound.scheduled.total_cycles + 2 ) as f64 ).log2().ceil() as u32;
    ffs  += state_bits;
    luts += state_bits * 2;

    for node in &dfg.nodes {
        match &node.op {
            DfgOp::Input(_) | DfgOp::Const(_) | DfgOp::Output(_) | DfgOp::Nop => {}
            DfgOp::Mux  => { luts += node.width; ffs += node.width; }
            _            => { ffs += node.width; }
        }
    }

    if luts > target.max_luts() {
        warnings.push( format!( "LUT usage {} exceeds {} capacity", luts, target.name() ) );
    }
    if dsps > target.max_dsps() {
        warnings.push( format!( "DSP usage {} exceeds target {}", dsps, target.max_dsps() ) );
    }

    let util = TargetUtilization {
        lut_pct:  luts as f64 / target.max_luts() as f64 * 100.0,
        ff_pct:   ffs as f64 / target.max_ffs() as f64 * 100.0,
        dsp_pct:  if target.max_dsps() > 0 { dsps as f64 / target.max_dsps() as f64 * 100.0 } else { 0.0 },
        bram_pct: if target.max_bram18() > 0 { brams as f64 / target.max_bram18() as f64 * 100.0 } else { 0.0 },
    };

    TargetReport {
        target_name:        target.name().to_string(),
        luts, ffs, dsps,
        bram18:             brams,
        bram36:             0,
        frequency_est_mhz:  target.max_freq_mhz(),
        utilization:        util,
        warnings,
    }
}

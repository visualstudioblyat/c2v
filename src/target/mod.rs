pub mod xilinx;
pub mod intel;
pub mod lattice;

use serde::Serialize;

use crate::ir::BoundDfg;

#[derive(Debug, Clone, Serialize)]
pub struct TargetReport {
    pub target_name: String,
    pub luts: u32,
    pub ffs: u32,
    pub dsps: u32,
    pub bram18: u32,
    pub bram36: u32,
    pub frequency_est_mhz: f64,
    pub utilization: TargetUtilization,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TargetUtilization {
    pub lut_pct: f64,
    pub ff_pct: f64,
    pub dsp_pct: f64,
    pub bram_pct: f64,
}

#[derive(Debug, Clone)]
pub struct TargetSpec {
    pub name:           String,
    pub max_freq_mhz:   f64,
    pub dsp_slices:     u32,
    pub bram_bits:      u64,
    pub add_delay_ns:   f64,
    pub mul_delay_ns:   f64,
    pub mux_delay_ns:   f64,
    pub route_delay_ns: f64,
}

impl TargetSpec {
    pub fn from_model( model: &dyn TargetModel ) -> Self {
        TargetSpec {
            name:           model.name().to_string(),
            max_freq_mhz:   model.max_freq_mhz(),
            dsp_slices:     model.max_dsps(),
            bram_bits:      model.max_bram18() as u64 * 18 * 1024,
            add_delay_ns:   1000.0 / model.max_freq_mhz() * 0.3,
            mul_delay_ns:   1000.0 / model.max_freq_mhz() * 0.7,
            mux_delay_ns:   1000.0 / model.max_freq_mhz() * 0.2,
            route_delay_ns: 1000.0 / model.max_freq_mhz() * 0.4,
        }
    }
}

pub trait TargetModel {
    fn name(&self) -> &str;
    fn estimate(&self, bound: &BoundDfg) -> TargetReport;
    fn max_luts(&self) -> u32;
    fn max_ffs(&self) -> u32;
    fn max_dsps(&self) -> u32;
    fn max_bram18(&self) -> u32;
    fn dsp_latency(&self) -> u32;
    fn bram_latency(&self) -> u32;
    fn max_freq_mhz(&self) -> f64;
}

pub fn get_target(name: &str) -> Option<Box<dyn TargetModel>> {
    match name {
        "xc7a35t" | "artix7" => Some(Box::new(xilinx::Artix7_35T)),
        "xc7a100t" => Some(Box::new(xilinx::Artix7_100T)),
        "xc7k325t" | "kintex7" => Some(Box::new(xilinx::Kintex7_325T)),
        "xcku040" | "ultrascale" => Some(Box::new(xilinx::UltraScalePlus_KU040)),
        "xcvu9p" | "alveo" => Some(Box::new(xilinx::UltraScalePlus_VU9P)),
        "5cefa5" | "cyclonev" => Some(Box::new(intel::CycloneV_5CEFA5)),
        "10ax115" | "arria10" => Some(Box::new(intel::Arria10_115)),
        "agfb014" | "agilex" => Some(Box::new(intel::Agilex_FB014)),
        "ice40up5k" | "ice40" => Some(Box::new(lattice::ICE40_UP5K)),
        "ice40hx8k" => Some(Box::new(lattice::ICE40_HX8K)),
        "ecp5-25f" | "ecp5" => Some(Box::new(lattice::ECP5_25F)),
        "ecp5-85f" => Some(Box::new(lattice::ECP5_85F)),
        "lifcl-17" | "crosslink" => Some(Box::new(lattice::CrossLink_NX_17)),
        _ => None,
    }
}

pub fn list_targets() -> Vec<(&'static str, &'static str)> {
    vec![
        ("xc7a35t",   "Xilinx Artix-7 35T (20k LUTs, 90 DSP)"),
        ("xc7a100t",  "Xilinx Artix-7 100T (63k LUTs, 240 DSP)"),
        ("xc7k325t",  "Xilinx Kintex-7 325T (203k LUTs, 840 DSP)"),
        ("xcku040",   "Xilinx UltraScale+ KU040 (242k LUTs, 1920 DSP)"),
        ("xcvu9p",    "Xilinx UltraScale+ VU9P (1182k LUTs, 6840 DSP) [Alveo]"),
        ("5cefa5",    "Intel Cyclone V 5CEFA5 (77k ALMs, 150 DSP)"),
        ("10ax115",   "Intel Arria 10 115 (427k ALMs, 1518 DSP)"),
        ("agfb014",   "Intel Agilex F B014 (487k ALMs, 3960 DSP)"),
        ("ice40up5k", "Lattice iCE40 UP5K (5k LUTs, 8 DSP)"),
        ("ice40hx8k", "Lattice iCE40 HX8K (7.6k LUTs, no DSP)"),
        ("ecp5-25f",  "Lattice ECP5 25F (24k LUTs, 28 DSP)"),
        ("ecp5-85f",  "Lattice ECP5 85F (84k LUTs, 156 DSP)"),
        ("lifcl-17",  "Lattice CrossLink-NX 17 (17k LUTs, 56 DSP)"),
    ]
}

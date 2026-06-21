use std::process::Command;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct SynthReport {
    pub target: String,
    pub cells: u32,
    pub wires: u32,
    pub luts: u32,
    pub ffs: u32,
    pub dsps: u32,
    pub brams: u32,
    pub raw_output: String,
}

pub fn run_yosys_synth(verilog_path: &str, target: &str) -> Result<SynthReport, String> {
    let synth_cmd = match target {
        "xilinx" | "xc7" | "artix7" | "kintex7" | "ultrascale" | "alveo" |
        "xc7a35t" | "xc7a100t" | "xc7k325t" | "xcku040" | "xcvu9p" => "synth_xilinx",
        "intel" | "altera" | "cyclonev" | "arria10" | "agilex" |
        "5cefa5" | "10ax115" | "agfb014" => "synth_intel",
        "ice40" => "synth_ice40",
        "ecp5" => "synth_ecp5",
        "generic" | _ => "synth",
    };

    let script = format!(
        "read_verilog {verilog_path}; {synth_cmd}; stat"
    );

    let output = Command::new("yosys")
        .args(["-p", &script])
        .output()
        .map_err(|e| format!("failed to run yosys: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        return Err(format!("yosys synthesis failed:\n{stderr}"));
    }

    let combined = format!("{stdout}\n{stderr}");

    // Parse the final stat block (after all optimizations)
    let cells = extract_stat_last(&combined, "cells");
    let wires = extract_stat_last(&combined, "wires");

    // Count LUTs by summing all LUT* cell types
    let luts = extract_cell_count(&combined, "LUT1")
        + extract_cell_count(&combined, "LUT2")
        + extract_cell_count(&combined, "LUT3")
        + extract_cell_count(&combined, "LUT4")
        + extract_cell_count(&combined, "LUT5")
        + extract_cell_count(&combined, "LUT6")
        + extract_cell_count(&combined, "SB_LUT4")
        + extract_cell_count(&combined, "$lut");
    let ffs = extract_cell_count(&combined, "FDRE")
        + extract_cell_count(&combined, "FDSE")
        + extract_cell_count(&combined, "FDCE")
        + extract_cell_count(&combined, "SB_DFF")
        + extract_cell_count(&combined, "$dff");
    let dsps = extract_cell_count(&combined, "DSP48")
        + extract_cell_count(&combined, "DSP48E1")
        + extract_cell_count(&combined, "DSP48E2");
    let brams = extract_cell_count(&combined, "RAMB18")
        + extract_cell_count(&combined, "RAMB36")
        + extract_cell_count(&combined, "SB_RAM40_4K");

    Ok(SynthReport {
        target: synth_cmd.to_string(),
        cells, wires, luts, ffs, dsps, brams,
        raw_output: combined,
    })
}

fn extract_stat_last(text: &str, key: &str) -> u32 {
    let mut result = 0;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.ends_with(key) || trimmed.contains(&format!(" {key}")) {
            for word in trimmed.split_whitespace() {
                if let Ok(n) = word.parse::<u32>() {
                    result = n;
                }
            }
        }
    }
    result
}

fn extract_cell_count(text: &str, cell_type: &str) -> u32 {
    let mut result = 0;
    for line in text.lines() {
        let trimmed = line.trim();
        // Format: "  323   IBUF" or "  224   LUT2"
        if trimmed.ends_with(cell_type) || trimmed.contains(&format!("   {cell_type}")) {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 2 && parts.last().map(|s| *s == cell_type).unwrap_or(false) {
                if let Ok(n) = parts[0].parse::<u32>() {
                    result = result.max(n);
                }
            }
        }
    }
    result
}

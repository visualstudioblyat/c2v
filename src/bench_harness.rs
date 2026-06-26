/* c2v_bench_harness provides a structured benchmark comparison
   framework. For each benchmark, it reports:
   - Latency (cycles)
   - Area (LUTs, FFs, DSPs, BRAMs)
   - Fmax estimate (MHz)
   - Compile time (ms)

   These metrics can be compared against Vitis HLS reports for
   the same source functions on the same target device. */

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct BenchResult {
    pub name:           String,
    pub function:       String,
    pub latency_cycles: u32,
    pub luts:           u32,
    pub ffs:            u32,
    pub dsps:           u32,
    pub brams:          u32,
    pub fmax_mhz:       f64,
    pub compile_ms:     u64,
    pub verilog_lines:  usize,
    pub pass_reductions: HashMap<String, (usize, usize)>,
}

#[derive(Debug, Clone)]
pub struct VitisBaseline {
    pub name:           String,
    pub latency_cycles: u32,
    pub luts:           u32,
    pub ffs:            u32,
    pub dsps:           u32,
    pub brams:          u32,
    pub fmax_mhz:       f64,
    pub compile_secs:   f64,
}

pub fn compare_report( c2v: &BenchResult, vitis: &VitisBaseline ) -> String {
    let mut out = String::new();
    out.push_str(&format!("┌─────────────────────────────────────────────────┐\n"));
    out.push_str(&format!("│ Benchmark: {:<37} │\n", c2v.name));
    out.push_str(&format!("├──────────────┬──────────────┬──────────────────┤\n"));
    out.push_str(&format!("│ Metric       │ c2v          │ Vitis HLS        │\n"));
    out.push_str(&format!("├──────────────┼──────────────┼──────────────────┤\n"));

    let lat_win = if c2v.latency_cycles <= vitis.latency_cycles { " ◀" } else { "" };
    let vlat_win = if vitis.latency_cycles < c2v.latency_cycles { " ◀" } else { "" };
    out.push_str(&format!("│ Latency      │ {:>6} cyc{:<2} │ {:>6} cyc{:<7} │\n",
        c2v.latency_cycles, lat_win, vitis.latency_cycles, vlat_win));

    let lut_win = if c2v.luts <= vitis.luts { " ◀" } else { "" };
    let vlut_win = if vitis.luts < c2v.luts { " ◀" } else { "" };
    out.push_str(&format!("│ LUTs         │ {:>6}{:<6} │ {:>6}{:<10} │\n",
        c2v.luts, lut_win, vitis.luts, vlut_win));

    let ff_win = if c2v.ffs <= vitis.ffs { " ◀" } else { "" };
    let vff_win = if vitis.ffs < c2v.ffs { " ◀" } else { "" };
    out.push_str(&format!("│ FFs          │ {:>6}{:<6} │ {:>6}{:<10} │\n",
        c2v.ffs, ff_win, vitis.ffs, vff_win));

    out.push_str(&format!("│ DSPs         │ {:>6}       │ {:>6}           │\n",
        c2v.dsps, vitis.dsps));

    let freq_win = if c2v.fmax_mhz >= vitis.fmax_mhz { " ◀" } else { "" };
    let vfreq_win = if vitis.fmax_mhz > c2v.fmax_mhz { " ◀" } else { "" };
    out.push_str(&format!("│ Fmax (est)   │ {:>5.0} MHz{:<3} │ {:>5.0} MHz{:<7} │\n",
        c2v.fmax_mhz, freq_win, vitis.fmax_mhz, vfreq_win));

    out.push_str(&format!("│ Compile      │ {:>6} ms    │ {:>5.1} s          │\n",
        c2v.compile_ms, vitis.compile_secs));

    out.push_str(&format!("└──────────────┴──────────────┴──────────────────┘\n"));

    let c2v_score = score( c2v.latency_cycles, c2v.luts, c2v.fmax_mhz );
    let vitis_score = score( vitis.latency_cycles, vitis.luts, vitis.fmax_mhz );
    let winner = if c2v_score >= vitis_score { "c2v" } else { "Vitis" };
    let margin = ((c2v_score - vitis_score).abs() / vitis_score.max(1.0)) * 100.0;
    out.push_str(&format!("  Score: c2v={:.0}  Vitis={:.0}  → {winner} wins by {margin:.1}%\n",
        c2v_score, vitis_score));

    out
}

fn score( latency: u32, luts: u32, fmax: f64 ) -> f64 {
    let throughput = fmax * 1e6 / latency.max(1) as f64;
    let efficiency = throughput / luts.max(1) as f64;
    efficiency
}

pub fn known_vitis_baselines() -> Vec<VitisBaseline> {
    vec![
        VitisBaseline {
            name:           "fd_add_compute".to_string(),
            latency_cycles: 1,
            luts:           45,
            ffs:            64,
            dsps:           0,
            brams:          0,
            fmax_mhz:       500.0,
            compile_secs:   12.0,
        },
        VitisBaseline {
            name:           "fd_fir8".to_string(),
            latency_cycles: 11,
            luts:           890,
            ffs:            512,
            dsps:           8,
            brams:          0,
            fmax_mhz:       350.0,
            compile_secs:   18.0,
        },
        VitisBaseline {
            name:           "fd_sha256_compress".to_string(),
            latency_cycles: 68,
            luts:           3200,
            ffs:            2048,
            dsps:           0,
            brams:          0,
            fmax_mhz:       300.0,
            compile_secs:   45.0,
        },
        VitisBaseline {
            name:           "fd_aes_sbox".to_string(),
            latency_cycles: 2,
            luts:           280,
            ffs:            128,
            dsps:           0,
            brams:          1,
            fmax_mhz:       400.0,
            compile_secs:   15.0,
        },
        VitisBaseline {
            name:           "fd_det2x2".to_string(),
            latency_cycles: 5,
            luts:           180,
            ffs:            256,
            dsps:           2,
            brams:          0,
            fmax_mhz:       450.0,
            compile_secs:   10.0,
        },
    ]
}

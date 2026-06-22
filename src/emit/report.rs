use std::fmt::Write;

use crate::ir::BoundDfg;
use crate::target::TargetReport;

pub fn emit_html_report(
    bound: &BoundDfg,
    _source: &str,
    target_report: Option<&TargetReport>,
    synth_cells: Option<u32>,
) -> String {
    let dfg = &bound.scheduled.dfg;
    let mut out = String::with_capacity(8192);

    writeln!(out, "<!DOCTYPE html>").unwrap();
    writeln!(out, "<html><head>").unwrap();
    writeln!(out, "<meta charset=\"utf-8\">").unwrap();
    writeln!(out, "<title>c2v report: {}</title>", dfg.name).unwrap();
    writeln!(out, "<style>").unwrap();
    writeln!(out, ":root {{ --bg:#070707; --fg:#aaa; --dim:#555; --accent:#a78bfa; --green:#3dd68c; --red:#ef4444; }}").unwrap();
    writeln!(out, "body {{ background:var(--bg); color:var(--fg); font:13px/1.6 'JetBrains Mono',monospace; max-width:800px; margin:40px auto; padding:0 20px; }}").unwrap();
    writeln!(out, "h1 {{ color:#e8e8e8; font-size:18px; font-weight:500; border-bottom:1px dashed #222; padding-bottom:12px; }}").unwrap();
    writeln!(out, "h2 {{ color:#ccc; font-size:14px; font-weight:400; margin-top:24px; }}").unwrap();
    writeln!(out, "table {{ border-collapse:collapse; width:100%; margin:12px 0; }}").unwrap();
    writeln!(out, "th,td {{ text-align:left; padding:4px 12px; border-bottom:1px solid #1a1a1a; }}").unwrap();
    writeln!(out, "th {{ color:var(--dim); font-weight:400; }}").unwrap();
    writeln!(out, ".num {{ text-align:right; font-variant-numeric:tabular-nums; }}").unwrap();
    writeln!(out, ".pass {{ color:var(--green); }}").unwrap();
    writeln!(out, ".fail {{ color:var(--red); }}").unwrap();
    writeln!(out, "pre {{ background:#0c0c0c; padding:12px; overflow-x:auto; border-left:2px solid #252525; }}").unwrap();
    writeln!(out, "code {{ color:#888; font-size:12px; }}").unwrap();
    writeln!(out, "</style></head><body>").unwrap();

    writeln!(out, "<h1>c2v &mdash; {}</h1>", dfg.name).unwrap();

    // Summary table
    writeln!(out, "<h2>Summary</h2>").unwrap();
    writeln!(out, "<table>").unwrap();
    writeln!(out, "<tr><td>Operations</td><td class=\"num\">{}</td></tr>", dfg.nodes.len()).unwrap();
    writeln!(out, "<tr><td>Edges</td><td class=\"num\">{}</td></tr>", dfg.edges.len()).unwrap();
    writeln!(out, "<tr><td>Latency</td><td class=\"num\">{} cycles</td></tr>", bound.scheduled.total_cycles).unwrap();
    writeln!(out, "<tr><td>Resources</td><td class=\"num\">{}</td></tr>", bound.resources.len()).unwrap();
    writeln!(out, "<tr><td>Inputs</td><td class=\"num\">{}</td></tr>", dfg.inputs.len()).unwrap();
    writeln!(out, "<tr><td>Outputs</td><td class=\"num\">{}</td></tr>", dfg.outputs.len()).unwrap();
    if !dfg.arrays.is_empty() {
        writeln!(out, "<tr><td>Arrays</td><td class=\"num\">{}</td></tr>", dfg.arrays.len()).unwrap();
    }
    if !dfg.structs.is_empty() {
        writeln!(out, "<tr><td>Structs</td><td class=\"num\">{}</td></tr>", dfg.structs.len()).unwrap();
    }
    if let Some(cells) = synth_cells {
        writeln!(out, "<tr><td>Synth cells</td><td class=\"num\">{cells}</td></tr>").unwrap();
    }
    writeln!(out, "</table>").unwrap();

    // Target report
    if let Some(tr) = target_report {
        writeln!(out, "<h2>Target: {}</h2>", tr.target_name).unwrap();
        writeln!(out, "<table>").unwrap();
        writeln!(out, "<tr><th>Resource</th><th class=\"num\">Used</th><th class=\"num\">%</th></tr>").unwrap();
        writeln!(out, "<tr><td>LUTs</td><td class=\"num\">{}</td><td class=\"num\">{:.2}%</td></tr>",
            tr.luts, tr.utilization.lut_pct).unwrap();
        writeln!(out, "<tr><td>FFs</td><td class=\"num\">{}</td><td class=\"num\">{:.2}%</td></tr>",
            tr.ffs, tr.utilization.ff_pct).unwrap();
        writeln!(out, "<tr><td>DSPs</td><td class=\"num\">{}</td><td class=\"num\">{:.2}%</td></tr>",
            tr.dsps, tr.utilization.dsp_pct).unwrap();
        writeln!(out, "<tr><td>BRAM18</td><td class=\"num\">{}</td><td class=\"num\">{:.2}%</td></tr>",
            tr.bram18, tr.utilization.bram_pct).unwrap();
        writeln!(out, "<tr><td>Est. freq</td><td class=\"num\" colspan=\"2\">{:.0} MHz</td></tr>",
            tr.frequency_est_mhz).unwrap();
        writeln!(out, "</table>").unwrap();
    }

    // Ports
    writeln!(out, "<h2>Ports</h2>").unwrap();
    writeln!(out, "<table>").unwrap();
    writeln!(out, "<tr><th>Name</th><th>Dir</th><th class=\"num\">Width</th></tr>").unwrap();
    for port in &dfg.inputs {
        writeln!(out, "<tr><td>{}</td><td>in</td><td class=\"num\">{}</td></tr>", port.name, port.width).unwrap();
    }
    for port in &dfg.outputs {
        writeln!(out, "<tr><td>{}</td><td>out</td><td class=\"num\">{}</td></tr>", port.name, port.width).unwrap();
    }
    writeln!(out, "</table>").unwrap();

    // Schedule
    writeln!(out, "<h2>Schedule ({} cycles)</h2>", bound.scheduled.total_cycles).unwrap();
    writeln!(out, "<table>").unwrap();
    writeln!(out, "<tr><th class=\"num\">Cycle</th><th class=\"num\">Ops</th></tr>").unwrap();
    for cycle in 0..bound.scheduled.total_cycles {
        let ops: Vec<_> = dfg.nodes.iter()
            .filter(|n| bound.scheduled.schedule.get(&n.id) == Some(&cycle))
            .filter(|n| !matches!(&n.op, crate::ir::DfgOp::Input(_) | crate::ir::DfgOp::Const(_) | crate::ir::DfgOp::Nop))
            .map(|n| n.source_name.as_str())
            .collect();
        if !ops.is_empty() {
            writeln!(out, "<tr><td class=\"num\">{cycle}</td><td>{}</td></tr>", ops.join(", ")).unwrap();
        }
    }
    writeln!(out, "</table>").unwrap();

    writeln!(out, "<footer style=\"margin-top:40px;color:var(--dim);font-size:11px;border-top:1px dashed #222;padding-top:12px;\">").unwrap();
    writeln!(out, "Generated by c2v 0.1.0</footer>").unwrap();
    writeln!(out, "</body></html>").unwrap();

    out
}

#![allow(dead_code)]

mod bind;
mod emit;
mod ir;
mod parse;
mod passes;
mod policy;
mod schedule;
mod standard;
mod synth;
mod target;
pub mod types;
mod verify;

use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use clap::Parser;

#[derive(Parser)]
#[command(name = "c2v", version, about = "Firedancer-style C to Verilog")]
struct Cli {
    /// Input C source file
    input: Option<PathBuf>,

    /// Output Verilog file
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Function to synthesize
    #[arg(short, long, default_value = "main")]
    function: String,

    /// Emit decision trace JSON
    #[arg(long)]
    trace: bool,

    /// Generate co-simulation testbench
    #[arg(long)]
    verify: bool,

    /// Skip coding standard validation (NOT recommended)
    #[arg(long)]
    no_validate: bool,

    /// Use simple ASAP scheduling instead of SDC
    #[arg(long)]
    asap: bool,

    /// Maximum multipliers available (resource constraint)
    #[arg(long, default_value = "4")]
    max_mul: u32,

    /// Maximum adders available (resource constraint)
    #[arg(long, default_value = "8")]
    max_add: u32,

    /// Emit pipelined Verilog (II=1, new input every clock)
    #[arg(long)]
    pipeline: bool,

    /// Balance reduction trees (Q1 Shannon limit optimization)
    #[arg(long)]
    balance: bool,

    /// Show parallelism entropy analysis (Q1 Shannon diagnostic)
    #[arg(long)]
    entropy: bool,

    /// Run C preprocessor (cc -E) before parsing
    #[arg(long)]
    preprocess: bool,

    /// Extra preprocessor flags (e.g. -I/path/to/headers)
    #[arg(long, short = 'D', num_args = 1)]
    define: Vec<String>,

    /// Target FPGA device for resource estimation
    #[arg(long, short = 'T')]
    target: Option<String>,

    /// List available target devices
    #[arg(long)]
    list_targets: bool,

    /// Run Yosys synthesis on the output (requires yosys)
    #[arg(long)]
    synth: bool,
}

fn main() {
    let cli = Cli::parse();

    if cli.list_targets {
        eprintln!("Available targets:");
        for (name, desc) in target::list_targets() {
            eprintln!("  {:<12} {}", name, desc);
        }
        return;
    }

    let input = cli.input.unwrap_or_else(|| {
        eprintln!("c2v: no input file specified");
        std::process::exit(1);
    });

    let t_start = Instant::now();

    let source = if cli.preprocess {
        let mut cmd = std::process::Command::new("cc");
        cmd.args(["-E", "-P", "-nostdinc"]);
        for d in &cli.define {
            cmd.arg(format!("-D{d}"));
        }
        cmd.arg(input.to_str().unwrap());
        let output = cmd.output().unwrap_or_else(|e| {
            eprintln!("c2v: failed to run preprocessor: {e}");
            std::process::exit(1);
        });
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("c2v: preprocessor failed:\n{stderr}");
            std::process::exit(1);
        }
        String::from_utf8_lossy(&output.stdout).to_string()
    } else {
        fs::read_to_string(&input).unwrap_or_else(|e| {
            eprintln!("c2v: cannot read {}: {e}", input.display());
            std::process::exit(1);
        })
    };

    let stem = input.file_stem().unwrap().to_str().unwrap();
    let out_path = cli.output.unwrap_or_else(|| PathBuf::from(format!("{stem}.v")));

    eprintln!("c2v 0.1.0 — Firedancer-style C to Verilog\n");

    // Phase 1: Parse
    let t0 = Instant::now();
    let tree = match parse::parse_c(&source) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("[parse]     FAILED: {e}");
            std::process::exit(1);
        }
    };
    let lines = source.lines().count();
    eprintln!("[parse]     {} ({lines} lines){:>20}", input.display(), format!("✓  {:?}", t0.elapsed()));

    // Phase 2: Validate coding standard
    if !cli.no_validate {
        let t0 = Instant::now();
        let violations = parse::validate_source(&tree, &source);
        if violations.is_empty() {
            eprintln!("[validate]  Firedancer standard{:>27}", format!("✓  {:?}", t0.elapsed()));
        } else {
            eprintln!("[validate]  REJECTED — {} violations:", violations.len());
            for v in &violations {
                eprintln!("            {}:{}: {}", input.display(), v.line, v.inner);
            }
            eprintln!("\n  Fix the source to conform to Firedancer C subset.");
            eprintln!("  See: https://github.com/firedancer-io/firedancer/blob/main/CONTRIBUTING.md");
            std::process::exit(1);
        }
    }

    // Phase 2.5: Build type table
    let t0 = Instant::now();
    let mut type_table = types::TypeTable::new();
    type_table.collect_from_tree(&tree, &source);
    let n_types = type_table.typedefs.len() + type_table.structs.len();
    let n_funcs = type_table.functions.len();
    eprintln!("[types]     {} types, {} functions{:>16}", n_types, n_funcs, format!("✓  {:?}", t0.elapsed()));

    // Phase 3: Build DFG
    let t0 = Instant::now();
    let dfg = match ir::build_dfg(&tree, &source, &cli.function) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[analyze]   FAILED: {e}");
            std::process::exit(1);
        }
    };
    let n_ops_raw = dfg.nodes.len();
    let n_edges_raw = dfg.edges.len();
    eprintln!("[analyze]   {n_ops_raw} ops, {n_edges_raw} edges{:>22}", format!("✓  {:?}", t0.elapsed()));

    // Entropy analysis (before optimization)
    let pre_report = if cli.entropy {
        let t0 = Instant::now();
        let report = ir::entropy::analyze_parallelism(&dfg);
        eprintln!("[entropy]   {:.1} bits redundancy ({:.0}% of ordering is free){:>2}",
            report.redundancy_bits,
            report.redundancy_ratio * 100.0,
            format!("✓  {:?}", t0.elapsed()));
        eprintln!("            depth={} width={} ops={} | W/D={:.1} | min_cycles={}",
            report.dependency_depth, report.max_antichain_width, report.total_ops,
            report.parallelism_upper_bound, report.theoretical_min_cycles);
        if !report.reduction_chains.is_empty() {
            eprintln!("            reduction chains:");
            for chain in &report.reduction_chains {
                eprintln!("              {:5} len={:<3} serial_depth={:<3} balanced_depth={:<3} recoverable={} ({:.1} bits)",
                    chain.op, chain.length, chain.length as u32 - 1, chain.balanced_depth,
                    chain.recoverable_depth, chain.redundancy_bits);
            }
            eprintln!("            predicted speedup from balancing: {:.2}x", report.predicted_speedup);
        }
        Some(report)
    } else {
        None
    };

    // Phase 3.5: Optimization pass pipeline
    let mut pipeline = passes::PassPipeline::new();
    pipeline.add(Box::new(passes::strength_reduce::StrengthReduction));
    pipeline.add(Box::new(passes::const_fold::ConstantFolding));
    pipeline.add(Box::new(passes::reassociate::Reassociate));
    pipeline.add(Box::new(passes::identity_elim::IdentityElimination));
    pipeline.add(Box::new(passes::mux_simplify::MuxSimplification));
    pipeline.add(Box::new(passes::cse::CommonSubexprElimination));
    pipeline.add(Box::new(passes::dead_node::DeadNodeElimination));
    pipeline.add(Box::new(passes::width_narrow::WidthNarrowing));

    let dfg = if cli.balance {
        let t0 = Instant::now();
        let balanced = ir::tree_balance::balance_reduction_trees(dfg);
        let n_new = balanced.nodes.iter().filter(|n| !matches!(n.op, ir::DfgOp::Nop)).count();
        eprintln!("[balance]   {} ops (depth reduced){:>14}", n_new, format!("✓  {:?}", t0.elapsed()));

        if cli.entropy {
            let post = ir::entropy::analyze_parallelism(&balanced);
            eprintln!("[entropy]   after: depth={} (was {}) | predicted {:.2}x, actual depth reduction {:.2}x",
                post.dependency_depth,
                pre_report.as_ref().map(|r| r.dependency_depth).unwrap_or(0),
                pre_report.as_ref().map(|r| r.predicted_speedup).unwrap_or(1.0),
                pre_report.as_ref().map(|r| r.dependency_depth as f64 / post.dependency_depth.max(1) as f64).unwrap_or(1.0),
            );
        }
        balanced
    } else {
        dfg
    };

    let dfg = pipeline.run(dfg).unwrap_or_else(|e| {
        eprintln!("[pass]      FAILED: {e}");
        std::process::exit(1);
    });
    let n_ops = dfg.nodes.len();

    // Phase 4: Schedule
    let t0 = Instant::now();
    let scheduled = if cli.asap {
        schedule::schedule_asap(dfg)
    } else {
        let rc = schedule::ResourceConstraints {
            max_adders: cli.max_add,
            max_multipliers: cli.max_mul,
            max_shifters: 8,
            max_comparators: 4,
        };
        schedule::schedule_sdc(dfg, &rc)
    };
    let sched_kind = if cli.asap { "ASAP" } else { "SDC" };
    eprintln!(
        "[schedule]  {} ops → {} cycles ({}){:>10}",
        n_ops,
        scheduled.total_cycles,
        sched_kind,
        format!("✓  {:?}", t0.elapsed())
    );

    // Phase 5: Bind
    let t0 = Instant::now();
    let bound = bind::bind_greedy(scheduled);
    let n_resources = bound.resources.len();
    eprintln!("[bind]      {n_resources} resources{:>26}", format!("✓  {:?}", t0.elapsed()));

    // Phase 6: Emit Verilog
    let t0 = Instant::now();
    let verilog = if cli.pipeline {
        emit::pipeline::emit_pipelined(&bound)
    } else {
        emit::emit_verilog(&bound)
    };
    let mode = if cli.pipeline { "pipelined II=1" } else { "FSM" };
    let v_lines = verilog.lines().count();
    fs::write(&out_path, &verilog).unwrap_or_else(|e| {
        eprintln!("[emit]      FAILED: {e}");
        std::process::exit(1);
    });
    eprintln!("[emit]      {} ({v_lines} lines, {mode}){:>6}", out_path.display(), format!("✓  {:?}", t0.elapsed()));

    // Phase 7: Decision trace
    if cli.trace {
        let t0 = Instant::now();
        let trace = policy::generate_trace(&bound);
        let trace_json = serde_json::to_string_pretty(&trace).unwrap();
        let trace_path = out_path.with_extension("trace.json");
        fs::write(&trace_path, &trace_json).unwrap();
        eprintln!("[trace]     {}{:>30}", trace_path.display(), format!("✓  {:?}", t0.elapsed()));
    }

    // Phase 8: Co-simulation verification
    if cli.verify {
        let t0 = Instant::now();
        let verify_path = out_path.with_extension("verify.v");
        if cli.pipeline {
            let verify_verilog = emit::emit_verilog(&bound);
            fs::write(&verify_path, &verify_verilog).unwrap();
        }
        let tb = verify::generate_testbench(&bound, &source);
        let tb_path = out_path.with_extension("tb.v");
        fs::write(&tb_path, &tb).unwrap();

        let sim_v = if cli.pipeline { verify_path.to_str().unwrap() } else { out_path.to_str().unwrap() };
        let sim_bin = format!("/tmp/c2v_sim_{}", std::process::id());
        let sim_result = std::process::Command::new("iverilog")
            .args(["-o", &sim_bin, sim_v, tb_path.to_str().unwrap()])
            .output();

        match sim_result {
            Ok(output) if output.status.success() => {
                let run = std::process::Command::new("vvp")
                    .arg(&sim_bin)
                    .output();
                match run {
                    Ok(run_out) => {
                        let stdout = String::from_utf8_lossy(&run_out.stdout);
                        if stdout.contains("PASS") {
                            eprintln!("[verify]    1000/1000 correct{:>21}", format!("✓  {:?}", t0.elapsed()));
                        } else if stdout.contains("MISMATCH") {
                            let mismatches = stdout.lines().filter(|l| l.contains("MISMATCH")).count();
                            eprintln!("[verify]    FAILED — {mismatches}/1000 mismatches");
                            for line in stdout.lines().filter(|l| l.contains("MISMATCH")).take(5) {
                                eprintln!("            {line}");
                            }
                            std::process::exit(1);
                        } else {
                            eprintln!("[verify]    simulation ran{:>23}", format!("✓  {:?}", t0.elapsed()));
                        }
                    }
                    Err(_) => {
                        eprintln!("[verify]    vvp not found — testbench at {}", tb_path.display());
                    }
                }
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                eprintln!("[verify]    iverilog compile failed:");
                for line in stderr.lines().take(5) {
                    eprintln!("            {line}");
                }
                std::process::exit(1);
            }
            Err(_) => {
                eprintln!("[verify]    iverilog not found — testbench at {}", tb_path.display());
            }
        }
        let _ = fs::remove_file(&sim_bin);
        if cli.pipeline {
            let _ = fs::remove_file(&verify_path);
        }
    }

    // Summary
    let area = policy::generate_trace(&bound).estimated_area;
    eprintln!("\nResults:");
    eprintln!("  Latency:     {} cycles", bound.scheduled.total_cycles);
    eprintln!("  Area est:    ~{} LUTs, {} FFs, {} DSP, {} BRAM", area.luts, area.ffs, area.dsps, area.brams);
    eprintln!("  Total time:  {:?}", t_start.elapsed());

    // Target-specific resource report
    if let Some(ref target_name) = cli.target {
        if let Some(tgt) = target::get_target(target_name) {
            let report = tgt.estimate(&bound);
            eprintln!("\n  Target: {}", report.target_name);
            eprintln!("  LUTs:  {:>6}  ({:.2}%)", report.luts, report.utilization.lut_pct);
            eprintln!("  FFs:   {:>6}  ({:.2}%)", report.ffs, report.utilization.ff_pct);
            eprintln!("  DSPs:  {:>6}  ({:.2}%)", report.dsps, report.utilization.dsp_pct);
            eprintln!("  BRAM18:{:>6}  ({:.2}%)", report.bram18, report.utilization.bram_pct);
            eprintln!("  Freq:  ~{:.0} MHz", report.frequency_est_mhz);
            for w in &report.warnings {
                eprintln!("  WARNING: {w}");
            }
        } else {
            eprintln!("\n  Unknown target: {target_name}. Use --list-targets to see available devices.");
        }
    }

    // Yosys synthesis
    if cli.synth {
        let t0 = Instant::now();
        let synth_target = cli.target.as_deref().unwrap_or("xilinx");
        match synth::run_yosys_synth(out_path.to_str().unwrap(), synth_target) {
            Ok(report) => {
                eprintln!("[synth]     {} → {} cells{:>16}",
                    report.target, report.cells, format!("✓  {:?}", t0.elapsed()));
                eprintln!("            LUTs: {}  FFs: {}  DSPs: {}  BRAMs: {}",
                    report.luts, report.ffs, report.dsps, report.brams);
            }
            Err(e) => {
                eprintln!("[synth]     FAILED: {e}");
            }
        }
    }
}

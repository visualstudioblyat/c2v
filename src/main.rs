#![allow(dead_code)]

mod bench;
mod bench_harness;
mod bind;
mod debug;
mod emit;
mod errors;
mod ir;
mod lint;
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
    /// Input C source file(s)
    input: Vec<PathBuf>,

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

    /// ML policy file (JSON) for schedule/bind overrides
    #[arg(long, short = 'P')]
    policy: Option<PathBuf>,

    /// Emit source map (C line → Verilog signal mapping)
    #[arg(long)]
    source_map: bool,

    /// Print DFG statistics
    #[arg(long)]
    stats: bool,

    /// Generate VCD waveform testbench
    #[arg(long)]
    wave: bool,

    /// Use graph coloring for resource binding (better sharing)
    #[arg(long)]
    graph_color: bool,

    /// Generate AXI-Stream wrapper
    #[arg(long)]
    axi: bool,

    /// Analyze loop pipelining potential (show II, recurrence, speedup)
    #[arg(long)]
    pipeline_analysis: bool,

    /// Show DSP48 inference results
    #[arg(long)]
    dsp: bool,

    /// Show critical path analysis
    #[arg(long)]
    critical_path: bool,

    /// Show array partitioning analysis
    #[arg(long)]
    array_partition: bool,

    /// Use Howard's algorithm for II computation (max-plus eigenvalue)
    #[arg(long)]
    howard: bool,

    /// Show Bjorklund non-integer II analysis
    #[arg(long)]
    bjorklund: bool,

    /// Emit as pipelined loop with iteration counter (parameterized N)
    #[arg(long)]
    loop_pipeline: bool,

    /// Trip count for loop pipeline (default 1024)
    #[arg(long, default_value = "1024")]
    trip_count: u32,

    /// Initiation interval for loop pipeline (default 1)
    #[arg(long, default_value = "1")]
    ii: u32,

    /// Use Bjorklund non-integer II: k resources, n operations (e.g. --bjorklund-ii 3,8)
    #[arg(long)]
    bjorklund_ii: Option<String>,

    /// Generate BRAM-ported accelerator wrapper (input_name:+output_name)
    #[arg(long)]
    accel: Option<String>,

    /// Generate streaming pipelined BRAM accelerator (input:output)
    #[arg(long)]
    stream: Option<String>,
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

    if cli.input.is_empty() {
        eprintln!("c2v: no input file specified");
        std::process::exit(1);
    }
    let input = cli.input[0].clone();

    let t_start = Instant::now();

    let source = if cli.preprocess {
        let mut combined = String::new();
        for path in &cli.input {
            let mut cmd = std::process::Command::new("cc");
            cmd.args(["-E", "-P", "-nostdinc"]);
            for d in &cli.define {
                cmd.arg(format!("-D{d}"));
            }
            cmd.arg(path.to_str().unwrap());
            let output = cmd.output().unwrap_or_else(|e| {
                eprintln!("c2v: failed to run preprocessor: {e}");
                std::process::exit(1);
            });
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                eprintln!("c2v: preprocessor failed on {}: {stderr}", path.display());
                std::process::exit(1);
            }
            combined.push_str(&String::from_utf8_lossy(&output.stdout));
            combined.push('\n');
        }
        combined
    } else {
        let mut combined = String::new();
        for path in &cli.input {
            let content = fs::read_to_string(path).unwrap_or_else(|e| {
                eprintln!("c2v: cannot read {}: {e}", path.display());
                std::process::exit(1);
            });
            combined.push_str(&content);
            combined.push('\n');
        }
        combined
    };

    let stem = if cli.input.len() > 1 {
        cli.function.as_str()
    } else {
        input.file_stem().unwrap().to_str().unwrap()
    };
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
    pipeline.add(Box::new(passes::constant_prop::ConstantPropagation));
    pipeline.add(Box::new(passes::loop_invariant::LoopInvariantCodeMotion));
    pipeline.add(Box::new(passes::chain_combine::ChainCombine));
    pipeline.add(Box::new(passes::duplicate_store::DuplicateStoreElimination));
    pipeline.add(Box::new(passes::dead_node::DeadNodeElimination));
    pipeline.add(Box::new(passes::copy_prop::CopyPropagation));
    pipeline.add(Box::new(passes::operator_merge::OperatorMerge));
    pipeline.add(Box::new(passes::share_mux::ShareMux));
    pipeline.add(Box::new(passes::retiming::Retiming));
    pipeline.add(Box::new(passes::loop_unroll::LoopUnroll));
    pipeline.add(Box::new(passes::const_fold::ConstantFolding));
    pipeline.add(Box::new(passes::dead_node::DeadNodeElimination));
    pipeline.add(Box::new(passes::width_narrow::WidthNarrowing));

    // Tree balancing is always on — reduces associative reduction
    // chain depth from O(n) to O(log n). The --balance flag now
    // only controls verbose output.
    let t0_bal = Instant::now();
    let dfg = ir::tree_balance::balance_reduction_trees(dfg);
    let n_balanced = dfg.nodes.iter().filter(|n| !matches!(n.op, ir::DfgOp::Nop)).count();
    if cli.balance || n_balanced != n_ops_raw {
        eprintln!("[balance]   {} ops (depth reduced){:>14}", n_balanced, format!("✓  {:?}", t0_bal.elapsed()));
    }

    if cli.entropy && cli.balance {
        let post = ir::entropy::analyze_parallelism(&dfg);
        eprintln!("[entropy]   after: depth={} (was {}) | predicted {:.2}x, actual depth reduction {:.2}x",
            post.dependency_depth,
            pre_report.as_ref().map(|r| r.dependency_depth).unwrap_or(0),
            pre_report.as_ref().map(|r| r.predicted_speedup).unwrap_or(1.0),
            pre_report.as_ref().map(|r| r.dependency_depth as f64 / post.dependency_depth.max(1) as f64).unwrap_or(1.0),
        );
    }

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

    // Phase 4.1: Memory port constraint check
    if !scheduled.dfg.arrays.is_empty() {
        let violations = schedule::mem_port::check_port_violations(
            &scheduled.dfg, &scheduled.schedule, 2,
        );
        if !violations.is_empty() {
            eprintln!("[memport]   WARNING: {} BRAM port violations", violations.len());
            for v in &violations {
                eprintln!("            array '{}' cycle {}: {} accesses (2 ports available)",
                    v.array, v.cycle, v.ports_needed);
            }
        }
    }

    // Phase 4.2: Howard's max-plus eigenvalue for II
    if cli.howard {
        let result = schedule::howard::howard_max_cycle_mean( &scheduled.dfg );
        eprintln!("[howard]    {}", schedule::howard::howard_report( &result ));
    }

    // Phase 4.25: Bjorklund non-integer II analysis
    if cli.bjorklund {
        let rc = schedule::ResourceConstraints {
            max_adders:      cli.max_add,
            max_multipliers: cli.max_mul,
            max_shifters:    8,
            max_comparators: 4,
        };
        let mut n_muls = 0u32;
        let mut n_adds = 0u32;
        for node in &scheduled.dfg.nodes {
            match &node.op {
                ir::DfgOp::Mul => n_muls += 1,
                ir::DfgOp::Add | ir::DfgOp::Sub => n_adds += 1,
                _ => {}
            }
        }
        if n_muls > rc.max_multipliers {
            let sched = schedule::bjorklund::bjorklund(rc.max_multipliers, n_muls);
            eprintln!("[bjorklund] MUL: {}", schedule::bjorklund::bjorklund_report(&sched));
        }
        if n_adds > rc.max_adders {
            let sched = schedule::bjorklund::bjorklund(rc.max_adders, n_adds);
            eprintln!("[bjorklund] ADD: {}", schedule::bjorklund::bjorklund_report(&sched));
        }
        if n_muls <= rc.max_multipliers && n_adds <= rc.max_adders {
            eprintln!("[bjorklund] no resource contention — integer II sufficient");
        }
    }

    // Phase 4.3: Critical path analysis
    if cli.critical_path {
        let cp = ir::critical_path::critical_path_summary( &scheduled.dfg );
        eprintln!("[crit]      {cp}");
    }

    // Phase 4.4: DSP inference
    if cli.dsp {
        let mappings = ir::dsp_infer::infer_dsp_patterns( &scheduled.dfg );
        let report = ir::dsp_infer::dsp_utilization_report( &mappings );
        eprintln!("[dsp]       {report}");
    }

    // Phase 4.45: Array partitioning
    if cli.array_partition {
        let plans = ir::array_partition::analyze_array_partitions( &scheduled.dfg );
        if !plans.is_empty() {
            eprintln!("[partition] {} arrays analyzed:", plans.len());
            eprint!("{}", ir::array_partition::partition_report( &plans ));
        }
    }

    // Phase 4.5: Loop pipeline analysis
    if cli.pipeline_analysis {
        let rc = schedule::ResourceConstraints {
            max_adders:      cli.max_add,
            max_multipliers: cli.max_mul,
            max_shifters:    8,
            max_comparators: 4,
        };
        let analysis = schedule::pipeline_loop::analyze_loop_pipeline(
            &scheduled.dfg, &[], &rc,
        );
        eprintln!("[pipeline]  body={} ops, latency={} cycles",
            analysis.body_ops, analysis.body_latency);
        eprintln!("            resource_II={}, recurrence_II={}, min_II={}",
            analysis.resource_ii, analysis.recurrence_ii, analysis.min_ii);
        if analysis.can_pipeline {
            let speedup = schedule::pipeline_emit::compute_pipeline_speedup(
                16, analysis.body_latency, analysis.min_ii,
            );
            eprintln!("            pipelineable: {:.1}x throughput for 16 iterations",
                speedup);
        } else {
            eprintln!("            {}", analysis.reason);
        }
        for dep in &analysis.loop_carried_deps {
            eprintln!("            carried: {} (latency={})", dep.var_name, dep.latency);
        }
    }

    // Phase 5: Bind
    let t0 = Instant::now();
    let bound = if cli.graph_color {
        bind::graph_color::bind_graph_coloring(scheduled)
    } else {
        bind::bind_greedy(scheduled)
    };
    let bind_kind = if cli.graph_color { "graph-color" } else { "greedy" };
    let n_resources = bound.resources.len();
    eprintln!("[bind]      {n_resources} resources ({bind_kind}){:>14}", format!("✓  {:?}", t0.elapsed()));

    // Lint
    let lint_warnings = lint::lint_dfg(&bound);
    if !lint_warnings.is_empty() {
        for w in &lint_warnings {
            let sev = match w.severity {
                lint::Severity::Info => "info",
                lint::Severity::Warning => "WARN",
                lint::Severity::Error => "ERR ",
            };
            eprintln!("[lint]      {sev}: {}", w.message);
        }
    }

    // Phase 6: Emit Verilog
    let t0 = Instant::now();
    let auto_loop = !bound.scheduled.dfg.loop_trip_counts.is_empty() && !cli.loop_pipeline;
    let use_loop_pipeline = cli.loop_pipeline || auto_loop;
    let loop_trip = if auto_loop {
        bound.scheduled.dfg.loop_trip_counts[0].1
    } else {
        cli.trip_count
    };
    if auto_loop {
        let (ref name, count) = bound.scheduled.dfg.loop_trip_counts[0];
        eprintln!("[auto]      loop '{}' ({} iterations) → pipeline mode", name, count);
    }
    let verilog = if use_loop_pipeline {
        let bjork = cli.bjorklund_ii.as_ref().and_then(|s| {
            let parts: Vec<&str> = s.split(',').collect();
            if parts.len() == 2 {
                Some((parts[0].parse::<u32>().ok()?, parts[1].parse::<u32>().ok()?))
            } else { None }
        });
        if let Some((k, n)) = bjork {
            let sched = schedule::bjorklund::bjorklund(k, n);
            eprintln!("[bjork]     non-integer II: {}", schedule::bjorklund::bjorklund_report(&sched));
        }
        let config = emit::loop_pipeline::LoopPipelineConfig {
            ii: cli.ii,
            trip_count: loop_trip,
            param_trip: true,
            bjorklund: bjork,
        };
        emit::loop_pipeline::emit_loop_pipeline(&bound, &config)
    } else if cli.pipeline {
        emit::pipeline::emit_pipelined(&bound)
    } else {
        emit::emit_verilog(&bound)
    };
    let mode = if use_loop_pipeline {
        format!("loop pipeline II={} N={}", cli.ii, loop_trip)
    } else if cli.pipeline {
        "pipelined II=1".to_string()
    } else {
        "FSM".to_string()
    };
    let v_lines = verilog.lines().count();
    fs::write(&out_path, &verilog).unwrap_or_else(|e| {
        eprintln!("[emit]      FAILED: {e}");
        std::process::exit(1);
    });
    eprintln!("[emit]      {} ({v_lines} lines, {mode}){:>6}", out_path.display(), format!("✓  {:?}", t0.elapsed()));

    // Phase 6.5: BRAM-ported accelerator wrapper
    if let Some(ref accel_spec) = cli.accel {
        let t0 = Instant::now();
        let parts: Vec<&str> = accel_spec.split(':').collect();
        let mut bram_ports = Vec::new();
        for part in &parts {
            let is_out = part.starts_with('+');
            let name = if is_out { &part[1..] } else { *part };
            bram_ports.push(emit::bram_port::BramPortSpec {
                name: name.to_string(),
                width: 32,
                depth: 1024,
                is_output: is_out,
            });
        }
        let accel_v = emit::bram_port::emit_bram_wrapper(&bound, &bram_ports);
        let accel_path = out_path.with_extension("accel.v");
        fs::write(&accel_path, &accel_v).unwrap();
        eprintln!("[accel]     {} ({} BRAM ports){:>10}",
            accel_path.display(), bram_ports.len(), format!("✓  {:?}", t0.elapsed()));
    }

    // Phase 6.6: Streaming pipelined BRAM accelerator
    if let Some(ref stream_spec) = cli.stream {
        let t0 = Instant::now();
        let parts: Vec<&str> = stream_spec.split(':').collect();
        let (inp, outp) = if parts.len() >= 2 { (parts[0], parts[1]) } else { ("input", "output") };
        let stream_v = emit::streaming_accel::emit_streaming_accelerator(&bound, inp, outp);
        let stream_path = out_path.with_extension("stream.v");
        fs::write(&stream_path, &stream_v).unwrap();
        eprintln!("[stream]    {} (II=1 pipelined BRAM){:>6}",
            stream_path.display(), format!("✓  {:?}", t0.elapsed()));
    }

    // Phase 6.7: AXI-Stream wrapper
    if cli.axi {
        let t0 = Instant::now();
        let axi_wrapper = emit::axi_stream::emit_axi_stream_wrapper(&bound);
        let axi_path = out_path.with_extension("axis.v");
        fs::write(&axi_path, &axi_wrapper).unwrap();
        eprintln!("[axi]       {}{:>30}", axi_path.display(), format!("✓  {:?}", t0.elapsed()));
    }

    // Phase 6.6: Waveform testbench
    if cli.wave {
        let wave_tb = emit::testbench_gen::generate_waveform_testbench(&bound);
        let wave_path = out_path.with_extension("wave.v");
        fs::write(&wave_path, &wave_tb).unwrap();
        eprintln!("[wave]      {}", wave_path.display());
    }

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

    // Source map
    if cli.source_map {
        let map_path = out_path.with_extension("map.json");
        let map = debug::generate_source_map(&bound);
        let map_json = serde_json::to_string_pretty(&map).unwrap();
        fs::write(&map_path, &map_json).unwrap();
        eprintln!("[map]       {}{:>30}", map_path.display(), format!("✓"));

        let comment = debug::emit_source_map_comment(&bound);
        let verilog_with_map = format!("{comment}\n{}", fs::read_to_string(&out_path).unwrap_or_default());
        let _ = fs::write(&out_path, verilog_with_map);
    }

    // Stats
    if cli.stats {
        let stats = debug::compute_stats(&bound);
        eprintln!("[stats]     {} active nodes, {} edges, fan-in={} fan-out={}",
            stats.active_nodes, stats.total_edges, stats.max_fan_in, stats.max_fan_out);
        eprintln!("            compute={} mux={} load={} store={} const={}",
            stats.compute_nodes, stats.mux_nodes, stats.load_nodes, stats.store_nodes, stats.const_nodes);
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

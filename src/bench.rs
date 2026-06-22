
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct BenchResult {
    pub name: String,
    pub ops: usize,
    pub edges: usize,
    pub cycles: u32,
    pub resources: usize,
    pub parse_us: u64,
    pub schedule_us: u64,
    pub emit_us: u64,
    pub verify_us: u64,
    pub total_us: u64,
    pub verified: bool,
    pub synth_cells: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct BenchSuite {
    pub results: Vec<BenchResult>,
    pub total_examples: usize,
    pub passed: usize,
    pub failed: usize,
    pub total_time_ms: u64,
}

pub fn format_bench_table(suite: &BenchSuite) -> String {
    let mut out = String::with_capacity(4096);

    out.push_str(&format!("{:<24} {:>6} {:>6} {:>5} {:>5} {:>8} {:>8}\n",
        "Example", "Ops", "Edges", "Cyc", "Res", "Emit μs", "Total μs"));
    out.push_str(&"-".repeat(72));
    out.push('\n');

    for r in &suite.results {
        let status = if r.verified { "✓" } else { "✗" };
        out.push_str(&format!("{:<24} {:>6} {:>6} {:>5} {:>5} {:>8} {:>7} {}\n",
            r.name, r.ops, r.edges, r.cycles, r.resources,
            r.emit_us, r.total_us, status));
    }

    out.push_str(&"-".repeat(72));
    out.push('\n');
    out.push_str(&format!("{} examples, {} passed, {} failed, {}ms total\n",
        suite.total_examples, suite.passed, suite.failed, suite.total_time_ms));

    out
}

pub fn format_bench_csv(suite: &BenchSuite) -> String {
    let mut out = String::with_capacity(4096);

    out.push_str("name,ops,edges,cycles,resources,parse_us,schedule_us,emit_us,verify_us,total_us,verified,synth_cells\n");

    for r in &suite.results {
        out.push_str(&format!("{},{},{},{},{},{},{},{},{},{},{},{}\n",
            r.name, r.ops, r.edges, r.cycles, r.resources,
            r.parse_us, r.schedule_us, r.emit_us, r.verify_us, r.total_us,
            r.verified,
            r.synth_cells.map(|c| c.to_string()).unwrap_or_default()));
    }

    out
}

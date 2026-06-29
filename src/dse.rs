use std::time::Instant;
use crate::ir::{Dfg, BoundDfg};
use crate::schedule::{self, ResourceConstraints};
use crate::bind;
use crate::policy;

#[derive(Debug, Clone)]
pub struct DsePoint {
    pub id:           usize,
    pub max_add:      u32,
    pub max_mul:      u32,
    pub max_shift:    u32,
    pub graph_color:  bool,
    pub latency:      u32,
    pub luts:         u32,
    pub ffs:          u32,
    pub dsps:         u32,
    pub area_time:    u64,
    pub compile_us:   u64,
}

#[derive(Debug)]
pub struct DseResult {
    pub points:       Vec<DsePoint>,
    pub pareto:       Vec<usize>,
    pub total_ms:     f64,
    pub best_latency: usize,
    pub best_area:    usize,
    pub best_product: usize,
}

pub fn explore_design_space(dfg: &Dfg, n_points: usize) -> DseResult {
    let t0 = Instant::now();
    let configs: Vec<(u32, u32, u32, bool)> = generate_configs(n_points);
    let mut points = Vec::with_capacity(configs.len());

    for (id, &(max_add, max_mul, max_shift, gc)) in configs.iter().enumerate() {
        let t_start = Instant::now();
        let rc = ResourceConstraints {
            max_adders: max_add,
            max_multipliers: max_mul,
            max_shifters: max_shift,
            max_comparators: 4,
        };
        let scheduled = schedule::schedule_sdc(dfg.clone(), &rc);
        let bound = if gc {
            bind::graph_color::bind_graph_coloring(scheduled)
        } else {
            bind::bind_greedy(scheduled)
        };
        let trace = policy::generate_trace(&bound);
        let area = &trace.estimated_area;
        let latency = bound.scheduled.total_cycles;
        let luts = area.luts;
        let ffs = area.ffs;
        let dsps = area.dsps;
        let at = latency as u64 * luts as u64;
        let elapsed = t_start.elapsed().as_micros() as u64;

        points.push(DsePoint {
            id, max_add, max_mul, max_shift, graph_color: gc,
            latency, luts, ffs, dsps, area_time: at, compile_us: elapsed,
        });
    }

    let pareto = compute_pareto(&points);
    let best_latency = points.iter().enumerate()
        .min_by_key(|(_, p)| p.latency).map(|(i, _)| i).unwrap_or(0);
    let best_area = points.iter().enumerate()
        .min_by_key(|(_, p)| p.luts).map(|(i, _)| i).unwrap_or(0);
    let best_product = points.iter().enumerate()
        .min_by_key(|(_, p)| p.area_time).map(|(i, _)| i).unwrap_or(0);
    let total_ms = t0.elapsed().as_secs_f64() * 1000.0;

    DseResult { points, pareto, total_ms, best_latency, best_area, best_product }
}

fn generate_configs(n: usize) -> Vec<(u32, u32, u32, bool)> {
    let mut configs = Vec::new();
    let add_range = [1, 2, 4, 8, 16];
    let mul_range = [1, 2, 4, 8];
    let shift_range = [1, 2, 4, 8];
    let gc_range = [false, true];

    for &a in &add_range {
        for &m in &mul_range {
            for &s in &shift_range {
                for &gc in &gc_range {
                    configs.push((a, m, s, gc));
                    if configs.len() >= n { return configs; }
                }
            }
        }
    }
    configs
}

fn compute_pareto(points: &[DsePoint]) -> Vec<usize> {
    let mut pareto = Vec::new();
    for (i, p) in points.iter().enumerate() {
        let dominated = points.iter().any(|q| {
            q.latency <= p.latency && q.luts <= p.luts && q.dsps <= p.dsps
                && (q.latency < p.latency || q.luts < p.luts || q.dsps < p.dsps)
        });
        if !dominated { pareto.push(i); }
    }
    pareto.sort_by_key(|&i| points[i].latency);
    pareto
}

pub fn print_dse_report(result: &DseResult) {
    eprintln!("[dse]       {} design points explored in {:.1}ms ({:.0} points/sec)",
        result.points.len(), result.total_ms,
        result.points.len() as f64 / (result.total_ms / 1000.0));
    eprintln!("[dse]       {} Pareto-optimal points:", result.pareto.len());
    eprintln!("[dse]       {:>4} {:>4} {:>4} {:>4} {:>4} {:>6} {:>8} {:>5}",
        "add", "mul", "shr", "bind", "cyc", "LUTs", "A×T", "µs");
    for &i in &result.pareto {
        let p = &result.points[i];
        let bind = if p.graph_color { "gc" } else { "gr" };
        let marker = if i == result.best_product { " ◀ best" } else { "" };
        eprintln!("[dse]       {:>4} {:>4} {:>4} {:>4} {:>4} {:>6} {:>8} {:>5}{}",
            p.max_add, p.max_mul, p.max_shift, bind,
            p.latency, p.luts, p.area_time, p.compile_us, marker);
    }
    let bp = &result.points[result.best_product];
    eprintln!("[dse]       best area×time: {} cycles, {} LUTs (A×T={})",
        bp.latency, bp.luts, bp.area_time);
}

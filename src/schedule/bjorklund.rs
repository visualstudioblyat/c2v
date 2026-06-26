/* c2v_bjorklund implements Bjorklund's algorithm for maximally-even
   distribution of k events in n time slots.

   Origin: Spallation Neutron Source timing system (Los Alamos, 2003).
   Equivalence: Euclidean algorithm on binary sequences (Toussaint 2005).

   HLS application: when the minimum II is non-integer (e.g., 3 ops
   need 2 resources → ideal II = 1.5), Bjorklund produces the
   maximally-even iteration start pattern. This achieves fractional
   throughput that fixed-integer-II schedulers (like Vitis) cannot.

   Example: E(3, 8) = [1,0,0,1,0,0,1,0] = Cuban tresillo rhythm
   For II=8/3: start iterations at cycles 0, 3, 5 (maximally even)

   The output is a binary sequence of length n with exactly k ones,
   where ones mark the cycles that start a new iteration. */

#[derive(Debug, Clone)]
pub struct BjorklundSchedule {
    pub pattern:       Vec<bool>,
    pub k:             u32,
    pub n:             u32,
    pub effective_ii:  f64,
    pub max_gap:       u32,
    pub min_gap:       u32,
    pub start_cycles:  Vec<u32>,
}

pub fn bjorklund( k: u32, n: u32 ) -> BjorklundSchedule {
    if k == 0 || n == 0 || k > n {
        return BjorklundSchedule {
            pattern: vec![false; n as usize],
            k, n,
            effective_ii: n as f64,
            max_gap: n,
            min_gap: n,
            start_cycles: Vec::new(),
        };
    }

    if k == n {
        return BjorklundSchedule {
            pattern: vec![true; n as usize],
            k, n,
            effective_ii: 1.0,
            max_gap: 1,
            min_gap: 1,
            start_cycles: (0..n).collect(),
        };
    }

    let pattern = euclidean_rhythm( k as usize, n as usize );

    let mut start_cycles = Vec::new();
    let mut gaps = Vec::new();
    let mut last_start = 0u32;

    for (i, &is_start) in pattern.iter().enumerate() {
        if is_start {
            if !start_cycles.is_empty() {
                gaps.push(i as u32 - last_start);
            }
            start_cycles.push(i as u32);
            last_start = i as u32;
        }
    }
    if start_cycles.len() > 1 {
        gaps.push(n - last_start + start_cycles[0]);
    }

    let max_gap = gaps.iter().copied().max().unwrap_or(n);
    let min_gap = gaps.iter().copied().min().unwrap_or(n);

    BjorklundSchedule {
        pattern,
        k, n,
        effective_ii: n as f64 / k as f64,
        max_gap,
        min_gap,
        start_cycles,
    }
}

fn euclidean_rhythm( k: usize, n: usize ) -> Vec<bool> {
    let mut groups: Vec<Vec<bool>> = Vec::new();

    for _ in 0..k {
        groups.push(vec![true]);
    }
    for _ in 0..(n - k) {
        groups.push(vec![false]);
    }

    loop {
        let ones_count = groups.iter()
            .filter(|g| g.first() == Some(&true))
            .count();
        let zeros_count = groups.len() - ones_count;

        if zeros_count <= 1 || ones_count <= 1 {
            break;
        }

        let min_count = ones_count.min(zeros_count);
        let mut new_groups = Vec::new();

        for i in 0..min_count {
            let mut combined = groups[i].clone();
            combined.extend_from_slice(&groups[groups.len() - 1 - i]);
            new_groups.push(combined);
        }

        let remaining_start = min_count;
        let remaining_end = groups.len() - min_count;
        for i in remaining_start..remaining_end {
            new_groups.push(groups[i].clone());
        }

        groups = new_groups;
    }

    groups.into_iter().flatten().collect()
}

pub fn compute_non_integer_ii(
    resource_ops: u32,
    available_units: u32,
) -> Option<BjorklundSchedule> {
    if available_units == 0 || resource_ops == 0 {
        return None;
    }

    let integer_ii = (resource_ops + available_units - 1) / available_units;
    let exact_slots = resource_ops;
    let total_slots = integer_ii * available_units;

    if exact_slots == total_slots {
        return None;
    }

    Some(bjorklund(available_units, exact_slots))
}

pub fn bjorklund_report( sched: &BjorklundSchedule ) -> String {
    let pattern_str: String = sched.pattern.iter()
        .map(|&b| if b { '1' } else { '0' })
        .collect();
    format!("E({},{}) = [{}] effective_II={:.2} gap=[{},{}] starts={:?}",
        sched.k, sched.n, pattern_str,
        sched.effective_ii, sched.min_gap, sched.max_gap,
        sched.start_cycles)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tresillo() {
        let s = bjorklund(3, 8);
        assert_eq!(s.k, 3);
        assert_eq!(s.n, 8);
        assert_eq!(s.pattern.iter().filter(|&&b| b).count(), 3);
        assert!(s.max_gap - s.min_gap <= 1);
    }

    #[test]
    fn test_equal() {
        let s = bjorklund(4, 4);
        assert_eq!(s.pattern, vec![true, true, true, true]);
        assert_eq!(s.effective_ii, 1.0);
    }

    #[test]
    fn test_single() {
        let s = bjorklund(1, 5);
        assert_eq!(s.pattern.iter().filter(|&&b| b).count(), 1);
        assert_eq!(s.effective_ii, 5.0);
    }

    #[test]
    fn test_maximally_even() {
        let s = bjorklund(5, 12);
        assert_eq!(s.pattern.iter().filter(|&&b| b).count(), 5);
        assert!(s.max_gap - s.min_gap <= 1,
            "not maximally even: gap range [{},{}]", s.min_gap, s.max_gap);
    }

    #[test]
    fn test_ii_fractional() {
        let s = bjorklund(3, 8);
        let ii = s.effective_ii;
        assert!((ii - 8.0/3.0).abs() < 0.01,
            "expected II=2.67, got {ii}");
    }
}

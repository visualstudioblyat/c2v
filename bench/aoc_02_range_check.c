/* AoC-style: check if sequence is "safe" (all increasing or
   all decreasing, adjacent diff 1-3). AoC 2024 Day 2.
   Tests: cascaded comparisons, logical AND chain.
   Metric: cycles to classify one report. */

typedef unsigned int uint;

uint
aoc_is_safe_4( uint a, uint b, uint c, uint d ) {
  uint ab = b - a;
  uint bc = c - b;
  uint cd = d - c;

  /* Check all increasing with diff 1-3 */
  uint inc_ab = (ab >= 1) & (ab <= 3);
  uint inc_bc = (bc >= 1) & (bc <= 3);
  uint inc_cd = (cd >= 1) & (cd <= 3);
  uint all_inc = inc_ab & inc_bc & inc_cd;

  /* Check all decreasing: a > b > c > d, diff 1-3 */
  uint ba = a - b;
  uint cb = b - c;
  uint dc = c - d;
  uint dec_ba = (ba >= 1) & (ba <= 3);
  uint dec_cb = (cb >= 1) & (cb <= 3);
  uint dec_dc = (dc >= 1) & (dc <= 3);
  uint all_dec = dec_ba & dec_cb & dec_dc;

  return all_inc | all_dec;
}

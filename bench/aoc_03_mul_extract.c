/* AoC-style: multiply-and-accumulate extracted pairs.
   Like AoC 2024 Day 3 (mul instructions).
   Tests: multiply + accumulate chain.
   Metric: throughput of mul-add pairs. */

typedef unsigned int uint;

uint
aoc_mul_sum_4( uint a0, uint b0, uint a1, uint b1,
               uint a2, uint b2, uint a3, uint b3 ) {
  return a0 * b0 + a1 * b1 + a2 * b2 + a3 * b3;
}

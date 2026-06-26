/* AoC-style: sum of 8 calibration values (parsed digits).
   Tests: array access + accumulation loop.
   Metric: cycles to produce sum, area. */

typedef unsigned int uint;

uint
aoc_sum_8( uint d0, uint d1, uint d2, uint d3,
           uint d4, uint d5, uint d6, uint d7 ) {
  return d0 + d1 + d2 + d3 + d4 + d5 + d6 + d7;
}

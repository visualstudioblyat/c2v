/* AoC-style: check if a pair satisfies an ordering rule.
   Like AoC 2024 Day 5 (page ordering).
   Tests: comparison + conditional logic.
   Metric: cycles per rule check. */

typedef unsigned int uint;

uint
aoc_check_order( uint before, uint after, uint pos_before, uint pos_after ) {
  uint valid = 0;
  if( pos_before < pos_after ) valid = 1;
  return valid;
}

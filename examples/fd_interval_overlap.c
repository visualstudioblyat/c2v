/* fd_interval_overlap — check if two intervals [a,b] and [c,d] overlap.
   AoC 2022 Day 4 style: range comparison.
   Tests: comparison chain → should be pure combinational.
   Returns overlap length (0 if no overlap). */

typedef unsigned int uint;

uint
fd_interval_overlap( uint a_lo, uint a_hi, uint b_lo, uint b_hi ) {
  uint start = (a_lo > b_lo) ? a_lo : b_lo;
  uint end   = (a_hi < b_hi) ? a_hi : b_hi;

  if( end > start ) {
    return end - start;
  }
  return 0;
}

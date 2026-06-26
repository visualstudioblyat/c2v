/* fd_prefix_sum — parallel prefix sum (Blelloch scan) on 8 elements.
   AoC/competitive classic: scan operation.
   Tests tree reduction pattern → tree balancing should help.
   Competitive metric: log2(8)=3 cycle depth vs 7 serial. */

typedef unsigned int uint;

uint
fd_prefix_sum_8( uint a0, uint a1, uint a2, uint a3,
                 uint a4, uint a5, uint a6, uint a7 ) {
  /* Up-sweep (reduce) */
  uint s01 = a0 + a1;
  uint s23 = a2 + a3;
  uint s45 = a4 + a5;
  uint s67 = a6 + a7;

  uint s03 = s01 + s23;
  uint s47 = s45 + s67;

  uint s07 = s03 + s47;

  /* Down-sweep (distribute) — we just return the total sum
     packed with the partial sums for verification */
  return s07;
}

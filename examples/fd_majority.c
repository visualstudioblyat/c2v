/* fd_majority — majority vote of 5 signals (TMR + 2).
   Tests: masking + addition + shift → pure combinational.
   Avoids comparisons for maximum portability. */

typedef unsigned int uint;

uint
fd_majority5( uint a, uint b, uint c, uint d, uint e ) {
  uint v0 = a & 1;
  uint v1 = b & 1;
  uint v2 = c & 1;
  uint v3 = d & 1;
  uint v4 = e & 1;
  uint count = v0 + v1 + v2 + v3 + v4;
  /* count is 0..5. Majority (>=3) iff bit 2 or bit 1 is set AND
     we haven't overflowed. Actually: count>=3 means count in {3,4,5}
     Binary: 011, 100, 101. Common: bit[2] | (bit[1] & bit[0]).
     But simpler: just (count >> 1) >> 1 gives 1 for 4,5 and
     (count >> 1) & 1 gives 1 for 2,3. So majority = bit2 | (bit1 & bit0).
     Even simpler: (count + count + count) >> 3 gives majority for 0..5.
     Nah. Cleanest: */
  uint bit2 = (count >> 2) & 1;
  uint bit1 = (count >> 1) & 1;
  uint bit0 = count & 1;
  return bit2 | (bit1 & bit0);
}

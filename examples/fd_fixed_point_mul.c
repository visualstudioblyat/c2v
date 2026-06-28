typedef unsigned int uint;

uint
fd_q16_mul( uint a, uint b ) {
  uint a_lo = a & 0xFFFF;
  uint a_hi = a >> 16;
  uint b_lo = b & 0xFFFF;
  uint b_hi = b >> 16;
  uint mid = a_lo * b_hi + a_hi * b_lo;
  uint hi  = a_hi * b_hi;
  uint result = hi + (mid >> 16);
  return result;
}

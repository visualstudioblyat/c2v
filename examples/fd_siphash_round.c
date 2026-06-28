typedef unsigned int uint;

uint
fd_sipround_lo( uint v0_lo, uint v0_hi, uint v1_lo, uint v1_hi ) {
  uint a_lo = v0_lo + v1_lo;
  uint carry = (a_lo < v0_lo) ? 1 : 0;
  uint a_hi = v0_hi + v1_hi + carry;
  uint rot_lo = (a_lo << 13) | (a_lo >> 19);
  uint rot_hi = (a_hi << 13) | (a_hi >> 19) | (a_lo >> 19);
  return rot_lo ^ rot_hi ^ a_lo ^ a_hi;
}

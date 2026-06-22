/* fd_ed25519_reduce — Ed25519 field reduction step (simplified).
   Tests: wide arithmetic + conditional subtract.
   Real-world: Solana signature verification. */

typedef unsigned int uint;

uint
fd_field_reduce( uint a_lo, uint a_hi, uint p_lo, uint p_hi ) {
  /* Compute a - p, check if a >= p */
  uint borrow;
  uint diff_lo;
  uint diff_hi;

  if( a_lo >= p_lo ) {
    diff_lo = a_lo - p_lo;
    borrow = 0;
  } else {
    diff_lo = a_lo + (0xFFFFFFFF - p_lo) + 1;
    borrow = 1;
  }

  if( a_hi >= p_hi + borrow ) {
    diff_hi = a_hi - p_hi - borrow;
    /* a >= p, return a - p */
    return (diff_hi << 16) | (diff_lo & 0xFFFF);
  } else {
    /* a < p, return a */
    return (a_hi << 16) | (a_lo & 0xFFFF);
  }
}

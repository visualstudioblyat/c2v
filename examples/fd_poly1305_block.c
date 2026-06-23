/* fd_poly1305_block — Poly1305 MAC accumulation step.
   Tests: wide multiply + add + modular reduction (simplified).
   Real-world: TLS 1.3 authentication (paired with ChaCha20). */

typedef unsigned int uint;

uint
fd_poly1305_block( uint acc_lo, uint acc_hi,
                   uint msg_lo, uint msg_hi,
                   uint r_lo,   uint r_hi ) {
  /* Add message block to accumulator */
  uint a_lo = acc_lo + msg_lo;
  uint carry = (a_lo < acc_lo) ? 1 : 0;
  uint a_hi = acc_hi + msg_hi + carry;

  /* Multiply by r (simplified: only low 32 bits of result) */
  uint prod_lo = a_lo * r_lo;
  uint prod_mid = a_lo * r_hi + a_hi * r_lo;
  uint prod_hi = a_hi * r_hi;

  /* Partial reduction mod 2^130-5 (simplified) */
  uint result_lo = prod_lo + (prod_hi * 5);
  uint result_hi = prod_mid + ((prod_hi * 5) >> 16);

  return result_lo ^ result_hi;
}

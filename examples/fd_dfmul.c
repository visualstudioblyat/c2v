/* fd_dfmul — integer-emulated floating point multiply (simplified).
   CHStone benchmark: dfmul (reduced to 16-bit mantissa for synthesis).
   Tests: wide multiply, shifts, if/else normalization. */

typedef unsigned int uint;

uint
fd_fp_mul( uint a, uint b ) {
  uint sign_a = (a >> 31) & 1;
  uint sign_b = (b >> 31) & 1;
  uint exp_a  = (a >> 23) & 0xFF;
  uint exp_b  = (b >> 23) & 0xFF;
  uint frac_a = (a & 0x7FFFFF) | 0x800000;
  uint frac_b = (b & 0x7FFFFF) | 0x800000;

  /* Result sign: XOR of input signs */
  uint sign_r = sign_a ^ sign_b;

  /* Result exponent: sum - bias */
  uint exp_r = exp_a + exp_b;
  if( exp_r >= 127 ) {
    exp_r = exp_r - 127;
  } else {
    exp_r = 0; /* underflow */
  }

  /* Multiply fractions (use upper 24 bits of product) */
  /* Truncate to 16-bit multiply to fit in 32-bit: frac >> 8 */
  uint fa = frac_a >> 8;
  uint fb = frac_b >> 8;
  uint product = fa * fb; /* 16x16 = 32 bit */

  /* Normalize */
  uint frac_r;
  if( product & 0x80000000 ) {
    frac_r = product >> 9;
    exp_r = exp_r + 1;
  } else {
    frac_r = product >> 8;
  }

  /* Clamp exponent */
  if( exp_r > 254 ) exp_r = 254;

  return (sign_r << 31) | ((exp_r & 0xFF) << 23) | (frac_r & 0x7FFFFF);
}

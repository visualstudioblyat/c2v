/* fd_dfadd — integer-emulated floating point addition (simplified).
   CHStone benchmark: dfadd (reduced to single-precision for demo).
   Tests: bitwise extraction, if/else, shifts, wide arithmetic. */

typedef unsigned int uint;

uint fd_fp_classify( uint a ) {
  uint exp = (a >> 23) & 0xFF;
  uint frac = a & 0x7FFFFF;
  if( exp == 0xFF ) {
    if( frac == 0 ) return 1; /* inf */
    return 2; /* nan */
  }
  if( exp == 0 && frac == 0 ) return 0; /* zero */
  return 3; /* normal */
}

uint
fd_fp_add( uint a, uint b ) {
  uint sign_a = (a >> 31) & 1;
  uint sign_b = (b >> 31) & 1;
  uint exp_a  = (a >> 23) & 0xFF;
  uint exp_b  = (b >> 23) & 0xFF;
  uint frac_a = (a & 0x7FFFFF) | 0x800000; /* add implicit 1 */
  uint frac_b = (b & 0x7FFFFF) | 0x800000;

  /* Align exponents */
  uint exp_diff;
  if( exp_a >= exp_b ) {
    exp_diff = (exp_a - exp_b) & 31;
    frac_b = frac_b >> exp_diff;
  } else {
    exp_diff = (exp_b - exp_a) & 31;
    frac_a = frac_a >> exp_diff;
    exp_a = exp_b;
    sign_a = sign_b;
  }

  /* Add or subtract fractions */
  uint frac_result;
  if( sign_a == sign_b ) {
    frac_result = frac_a + frac_b;
  } else {
    if( frac_a >= frac_b ) {
      frac_result = frac_a - frac_b;
    } else {
      frac_result = frac_b - frac_a;
      sign_a = sign_b;
    }
  }

  /* Normalize: shift right if overflow */
  uint exp_result = exp_a;
  if( frac_result & 0x1000000 ) {
    frac_result = frac_result >> 1;
    exp_result = exp_result + 1;
  }

  /* Pack result */
  return (sign_a << 31) | ((exp_result & 0xFF) << 23) | (frac_result & 0x7FFFFF);
}

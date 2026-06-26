/* fd_roman_numeral — convert integer to Roman numeral encoding.
   AoC 2023+ style: string/encoding problem adapted for hardware.
   Encodes value (0-3999) as packed 4-bit symbols.
   Tests cascaded if/else → mux chain, strength reduction. */

typedef unsigned int uint;

uint
fd_roman_encode( uint val ) {
  uint result = 0;
  uint shift  = 0;

  /* Thousands */
  uint th = val / 1000;
  result = result | (th << shift);
  shift  = shift + 4;
  val    = val - th * 1000;

  /* Hundreds */
  uint hu = val / 100;
  result = result | (hu << shift);
  shift  = shift + 4;
  val    = val - hu * 100;

  /* Tens */
  uint te = val / 10;
  result = result | (te << shift);
  shift  = shift + 4;
  val    = val - te * 10;

  /* Ones */
  result = result | (val << shift);

  return result;
}

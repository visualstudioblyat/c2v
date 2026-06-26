/* fd_base_convert — extract decimal digits from a 16-bit number.
   AoC classic: number → digits for display or checksum.
   Tests: repeated division → strength reduction opportunity.
   Returns packed BCD (4 bits per digit, 5 digits max). */

typedef unsigned int uint;

uint
fd_to_bcd( uint val ) {
  val = val & 0xFFFF;
  uint d4 = val / 10000;
  val = val - d4 * 10000;
  uint d3 = val / 1000;
  val = val - d3 * 1000;
  uint d2 = val / 100;
  val = val - d2 * 100;
  uint d1 = val / 10;
  uint d0 = val - d1 * 10;

  return (d4 << 16) | (d3 << 12) | (d2 << 8) | (d1 << 4) | d0;
}

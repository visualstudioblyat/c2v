/* fd_fixed_sqrt — fixed-point square root via restoring algorithm.
   Tests: loop + if/else + shift + subtract.
   Real-world: DSP, graphics, distance computation. */

typedef unsigned int uint;

uint
fd_fixed_sqrt( uint x ) {
  uint result = 0;
  uint bit = 1 << 30;
  uint num = x;

  for( int i = 0; i < 16; i++ ) {
    uint trial = result + bit;
    if( num >= trial ) {
      num = num - trial;
      result = (result >> 1) + bit;
    } else {
      result = result >> 1;
    }
    bit = bit >> 2;
  }
  return result;
}

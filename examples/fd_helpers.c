/* fd_helpers — helper functions in a separate file. */

typedef unsigned int uint;

uint fd_sat_add( uint a, uint b ) {
  uint sum = a + b;
  if( sum < a ) return 0xFFFFFFFF;
  return sum;
}

uint fd_sat_sub( uint a, uint b ) {
  if( b > a ) return 0;
  return a - b;
}

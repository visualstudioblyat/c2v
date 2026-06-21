/* fd_abs_diff — if/else with arithmetic in branches. */

typedef unsigned int uint;

uint
fd_abs_diff( uint a, uint b ) {
  uint result;
  if( a >= b ) {
    result = a - b;
  } else {
    result = b - a;
  }
  return result;
}

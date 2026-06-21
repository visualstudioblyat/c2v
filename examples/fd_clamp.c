/* fd_clamp — nested if/else → chained muxes. */

typedef unsigned int uint;

uint
fd_clamp( uint val, uint lo, uint hi ) {
  uint result;
  if( val < lo ) {
    result = lo;
  } else if( val > hi ) {
    result = hi;
  } else {
    result = val;
  }
  return result;
}

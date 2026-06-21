/* fd_max — if/else → mux. Basic control flow. */

typedef unsigned int uint;

uint
fd_max( uint a, uint b ) {
  uint result;
  if( a > b ) {
    result = a;
  } else {
    result = b;
  }
  return result;
}

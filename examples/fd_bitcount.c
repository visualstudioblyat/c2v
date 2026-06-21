/* fd_bitcount — population count via loop + conditional.
   Tests loop + if/else interaction. */

typedef unsigned int uint;

uint
fd_bitcount( uint x ) {
  uint count = 0;
  for( int i = 0; i < 32; i++ ) {
    if( (x >> i) & 1 ) {
      count += 1;
    }
  }
  return count;
}

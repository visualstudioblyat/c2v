/* fd_priority_enc — priority encoder via loop + if.
   Finds the highest set bit position. Common FPGA primitive. */

typedef unsigned int uint;

uint
fd_priority_enc( uint x ) {
  uint pos = 0;
  for( int i = 31; i >= 0; i-- ) {
    if( (x >> i) & 1 ) {
      pos = i;
    }
  }
  return pos;
}

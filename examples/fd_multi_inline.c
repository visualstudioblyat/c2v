/* fd_multi_inline — chain of 3 inlined functions.
   Tests deep inlining: f calls g calls h. */

typedef unsigned int uint;

uint fd_rotl( uint x, uint n ) {
  return (x << n) | (x >> (32 - n));
}

uint fd_quarter_round( uint a, uint b ) {
  a += b;
  a = fd_rotl( a, 7 );
  a ^= b;
  return a;
}

uint fd_double_round( uint a, uint b, uint c ) {
  uint r1 = fd_quarter_round( a, b );
  uint r2 = fd_quarter_round( r1, c );
  return r2;
}

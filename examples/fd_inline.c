/* fd_inline — multi-function with inlining + compound assignment */

typedef unsigned int uint;

uint fd_hash_mix( uint x ) {
  x ^= x >> 16;
  x *= 0x45d9f3b;
  x ^= x >> 16;
  return x;
}

uint fd_hash2( uint a, uint b ) {
  uint h = fd_hash_mix( a );
  h += fd_hash_mix( b );
  return h;
}

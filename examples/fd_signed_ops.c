typedef unsigned int uint;

int
fd_sign( int x ) {
  if( x > 0 ) return 1;
  if( x < 0 ) return -1;
  return 0;
}

int
fd_clamp_signed( int val, int lo, int hi ) {
  if( val < lo ) return lo;
  if( val > hi ) return hi;
  return val;
}

int
fd_lerp_fixed( int a, int b, uint t ) {
  int diff = b - a;
  int scaled = (diff * (int)(t & 0xFF)) >> 8;
  return a + scaled;
}

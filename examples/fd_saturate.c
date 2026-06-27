typedef unsigned int uint;

uint
fd_saturate_add( uint a, uint b ) {
  uint sum = a + b;
  if( sum < a ) return 0xFFFFFFFF;
  return sum;
}

uint
fd_saturate_sub( uint a, uint b ) {
  if( b > a ) return 0;
  return a - b;
}

uint
fd_clamp_u16( uint val, uint lo, uint hi ) {
  if( val < lo ) return lo;
  if( val > hi ) return hi;
  return val;
}

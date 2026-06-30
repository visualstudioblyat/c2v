typedef unsigned int uint;

uint
fd_stage1( uint x ) {
  return (x * x) >> 8;
}

uint
fd_stage2( uint x ) {
  return x + (x >> 1) + 42;
}

uint
fd_two_stage( uint x ) {
  uint mid = fd_stage1( x );
  return fd_stage2( mid );
}

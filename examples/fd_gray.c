/* fd_gray — Gray code encoder/decoder.
   Tests: simple bitwise, useful for FPGA state machines. */

typedef unsigned int uint;

uint fd_to_gray( uint binary ) {
  return binary ^ (binary >> 1);
}

uint fd_from_gray( uint gray ) {
  uint binary = gray;
  binary ^= (binary >> 16);
  binary ^= (binary >> 8);
  binary ^= (binary >> 4);
  binary ^= (binary >> 2);
  binary ^= (binary >> 1);
  return binary;
}

uint
fd_gray_roundtrip( uint x ) {
  uint g = fd_to_gray( x );
  uint b = fd_from_gray( g );
  return b;
}

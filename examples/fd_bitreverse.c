/* fd_bitreverse — reverse bits of a 32-bit integer.
   AoC-style: bit manipulation puzzle.
   Tests: shift + mask + or chain → tree balanceable.
   Competitive metric: should be 1 cycle combinational. */

typedef unsigned int uint;

uint
fd_bitreverse32( uint x ) {
  x = ((x & 0x55555555) << 1)  | ((x >> 1)  & 0x55555555);
  x = ((x & 0x33333333) << 2)  | ((x >> 2)  & 0x33333333);
  x = ((x & 0x0F0F0F0F) << 4)  | ((x >> 4)  & 0x0F0F0F0F);
  x = ((x & 0x00FF00FF) << 8)  | ((x >> 8)  & 0x00FF00FF);
  x = (x << 16) | (x >> 16);
  return x;
}

/* fd_hamming — Hamming distance between two 32-bit words.
   Tests: XOR + popcount pipeline.
   Real-world: error detection, similarity matching, DNA analysis. */

typedef unsigned int uint;

uint
fd_hamming_distance( uint a, uint b ) {
  uint x = a ^ b;

  /* Popcount using SWAR (SIMD within a register) */
  x = x - ((x >> 1) & 0x55555555);
  x = (x & 0x33333333) + ((x >> 2) & 0x33333333);
  x = (x + (x >> 4)) & 0x0F0F0F0F;
  x = x + (x >> 8);
  x = x + (x >> 16);

  return x & 0x3F;
}

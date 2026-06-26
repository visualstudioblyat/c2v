/* fd_hamming_dist — Hamming distance between two 32-bit values.
   AoC-style: bit manipulation problem. XOR + popcount.
   Competitive metric: should be 1 cycle (pure combinational). */

typedef unsigned int uint;

uint
fd_hamming_dist( uint a, uint b ) {
  uint x = a ^ b;
  uint count = 0;
  count = count + (x & 1); x = x >> 1;
  count = count + (x & 1); x = x >> 1;
  count = count + (x & 1); x = x >> 1;
  count = count + (x & 1); x = x >> 1;
  count = count + (x & 1); x = x >> 1;
  count = count + (x & 1); x = x >> 1;
  count = count + (x & 1); x = x >> 1;
  count = count + (x & 1); x = x >> 1;
  count = count + (x & 1); x = x >> 1;
  count = count + (x & 1); x = x >> 1;
  count = count + (x & 1); x = x >> 1;
  count = count + (x & 1); x = x >> 1;
  count = count + (x & 1); x = x >> 1;
  count = count + (x & 1); x = x >> 1;
  count = count + (x & 1); x = x >> 1;
  count = count + (x & 1); x = x >> 1;
  count = count + (x & 1); x = x >> 1;
  count = count + (x & 1); x = x >> 1;
  count = count + (x & 1); x = x >> 1;
  count = count + (x & 1); x = x >> 1;
  count = count + (x & 1); x = x >> 1;
  count = count + (x & 1); x = x >> 1;
  count = count + (x & 1); x = x >> 1;
  count = count + (x & 1); x = x >> 1;
  count = count + (x & 1); x = x >> 1;
  count = count + (x & 1); x = x >> 1;
  count = count + (x & 1); x = x >> 1;
  count = count + (x & 1); x = x >> 1;
  count = count + (x & 1); x = x >> 1;
  count = count + (x & 1); x = x >> 1;
  count = count + (x & 1); x = x >> 1;
  count = count + (x & 1);
  return count;
}

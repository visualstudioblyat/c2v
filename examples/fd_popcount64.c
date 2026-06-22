/* fd_popcount64 — 64-bit population count via Hamming weight.
   Tests: bitwise tricks + wide arithmetic.
   Real-world: networking (packet classification), crypto. */

typedef unsigned long ulong;
typedef unsigned int uint;

uint
fd_popcount64( ulong x ) {
  ulong v = x;
  v = v - ((v >> 1) & 0x5555555555555555);
  v = (v & 0x3333333333333333) + ((v >> 2) & 0x3333333333333333);
  v = (v + (v >> 4)) & 0x0f0f0f0f0f0f0f0f;
  v = v + (v >> 8);
  v = v + (v >> 16);
  v = v + (v >> 32);
  uint result = v & 0x7f;
  return result;
}

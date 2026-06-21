/* fd_dot4 — 4-element dot product using arrays.
   Two array reads per iteration, multiply-accumulate. */

typedef unsigned int uint;

uint
fd_dot4_compute( uint a0, uint a1, uint a2, uint a3,
                 uint b0, uint b1, uint b2, uint b3 ) {
  uint a[4] = { a0, a1, a2, a3 };
  uint b[4] = { b0, b1, b2, b3 };

  uint sum = a[0] * b[0];
  sum = sum + a[1] * b[1];
  sum = sum + a[2] * b[2];
  sum = sum + a[3] * b[3];
  return sum;
}

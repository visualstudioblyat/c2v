/* fd_fir8_loop — 8-tap FIR filter using a for-loop.
   Same computation as fd_fir8_parallel but expressed as a loop.
   The compiler unrolls it and finds parallelism. */

typedef unsigned int uint;

uint
fd_fir8_loop( uint x0, uint x1, uint x2, uint x3,
              uint x4, uint x5, uint x6, uint x7,
              uint c0, uint c1, uint c2, uint c3,
              uint c4, uint c5, uint c6, uint c7 ) {
  uint x[8] = { x0, x1, x2, x3, x4, x5, x6, x7 };
  uint c[8] = { c0, c1, c2, c3, c4, c5, c6, c7 };
  uint acc = 0;
  for( int i = 0; i < 8; i++ ) {
    acc = acc + x[i] * c[i];
  }
  return acc;
}

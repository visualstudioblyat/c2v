/* fd_fir8 — 8-tap FIR filter, Firedancer style
   Fixed coefficient array, bounded loop, pure computation. */

typedef unsigned int uint;

uint
fd_fir8_compute( uint x0,
                 uint x1,
                 uint x2,
                 uint x3,
                 uint x4,
                 uint x5,
                 uint x6,
                 uint x7,
                 uint c0,
                 uint c1,
                 uint c2,
                 uint c3,
                 uint c4,
                 uint c5,
                 uint c6,
                 uint c7 ) {
  uint acc = x0 * c0;
  acc = acc + x1 * c1;
  acc = acc + x2 * c2;
  acc = acc + x3 * c3;
  acc = acc + x4 * c4;
  acc = acc + x5 * c5;
  acc = acc + x6 * c6;
  acc = acc + x7 * c7;
  return acc;
}

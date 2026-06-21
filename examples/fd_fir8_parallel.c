/* fd_fir8_parallel — 8-tap FIR written for maximum parallelism.
   All multiplies are independent. Adder tree reduces in log2 stages. */

typedef unsigned int uint;

uint
fd_fir8_parallel( uint x0, uint x1, uint x2, uint x3,
                  uint x4, uint x5, uint x6, uint x7,
                  uint c0, uint c1, uint c2, uint c3,
                  uint c4, uint c5, uint c6, uint c7 ) {
  /* Stage 1: 8 independent multiplies */
  uint m0 = x0 * c0;
  uint m1 = x1 * c1;
  uint m2 = x2 * c2;
  uint m3 = x3 * c3;
  uint m4 = x4 * c4;
  uint m5 = x5 * c5;
  uint m6 = x6 * c6;
  uint m7 = x7 * c7;

  /* Stage 2: 4 adds */
  uint a0 = m0 + m1;
  uint a1 = m2 + m3;
  uint a2 = m4 + m5;
  uint a3 = m6 + m7;

  /* Stage 3: 2 adds */
  uint b0 = a0 + a1;
  uint b1 = a2 + a3;

  /* Stage 4: final add */
  return b0 + b1;
}

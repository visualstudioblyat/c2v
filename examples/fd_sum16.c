/* fd_sum16 — sum 16 elements via loop.
   Tests basic loop unrolling + adder tree optimization. */

typedef unsigned int uint;

uint
fd_sum16( uint a0,  uint a1,  uint a2,  uint a3,
          uint a4,  uint a5,  uint a6,  uint a7,
          uint a8,  uint a9,  uint a10, uint a11,
          uint a12, uint a13, uint a14, uint a15 ) {
  uint v[16] = { a0, a1, a2, a3, a4, a5, a6, a7,
                 a8, a9, a10, a11, a12, a13, a14, a15 };
  uint sum = 0;
  for( int i = 0; i < 16; i++ ) {
    sum = sum + v[i];
  }
  return sum;
}

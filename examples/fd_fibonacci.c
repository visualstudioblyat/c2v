/* fd_fibonacci — compute N-th Fibonacci number (mod 2^32).
   AoC-style: classic dynamic programming / recurrence.
   Tests loop-carried dependency (sum = a + b, then shift).
   Competitive metric: II=1 impossible due to recurrence,
   but area should be minimal (2 regs + 1 adder). */

typedef unsigned int uint;

uint
fd_fibonacci( uint n ) {
  uint a = 0;
  uint b = 1;
  for( int i = 0; i < 32; i++ ) {
    if( (uint)i >= n ) break;
    uint tmp = a + b;
    a = b;
    b = tmp;
  }
  return a;
}

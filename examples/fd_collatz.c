/* fd_collatz — Collatz sequence step counter (max 64 steps).
   AoC-style: number theory problem appearing in competitive
   programming. Tests conditional loop with data-dependent exit.
   Competitive metric: throughput (one query per clock ideal). */

typedef unsigned int uint;

uint
fd_collatz_steps( uint n ) {
  uint steps = 0;
  for( int i = 0; i < 64; i++ ) {
    if( n <= 1 ) break;
    if( n & 1 ) {
      n = 3 * n + 1;
    } else {
      n = n >> 1;
    }
    steps = steps + 1;
  }
  return steps;
}

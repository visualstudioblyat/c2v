/* fd_gcd — Euclidean GCD via subtraction (no division).
   AoC-style: classic algorithmic problem, tests loop + branch.
   Competitive metric: cycles to convergence, area. */

typedef unsigned int uint;

uint
fd_gcd( uint a, uint b ) {
  for( int i = 0; i < 32; i++ ) {
    if( a == b ) break;
    if( a > b ) {
      a = a - b;
    } else {
      b = b - a;
    }
  }
  return a;
}

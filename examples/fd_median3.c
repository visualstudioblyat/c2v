/* fd_median3 — sorting network for 3 elements.
   Tests: multiple if/else, variable swaps, inlined helper. */

typedef unsigned int uint;

uint fd_swap_if_gt( uint a, uint b ) {
  uint r = a;
  if( a > b ) { r = b; }
  return r;
}

uint fd_swap_if_lt( uint a, uint b ) {
  uint r = b;
  if( a > b ) { r = a; }
  return r;
}

uint
fd_median3( uint a, uint b, uint c ) {
  /* compare-swap network */
  uint lo_ab = fd_swap_if_gt( a, b );
  uint hi_ab = fd_swap_if_lt( a, b );
  uint lo_bc = fd_swap_if_gt( hi_ab, c );
  uint mid   = fd_swap_if_lt( lo_ab, lo_bc );
  return mid;
}

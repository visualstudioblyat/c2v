/* fd_sort4 — sorting network for 4 elements (5 compare-swaps).
   AoC-style: sorting problem → optimal comparison network.
   Tests: compare + mux chains → parallel compare-swap stages.
   Competitive metric: 3 stages (depth 3), 5 comparators. */

typedef unsigned int uint;

uint
fd_sort4_packed( uint a, uint b, uint c, uint d ) {
  uint t;

  /* Stage 1: compare-swap (a,c) and (b,d) — parallel */
  if( a > c ) { t = a; a = c; c = t; }
  if( b > d ) { t = b; b = d; d = t; }

  /* Stage 2: compare-swap (a,b) and (c,d) — parallel */
  if( a > b ) { t = a; a = b; b = t; }
  if( c > d ) { t = c; c = d; d = t; }

  /* Stage 3: compare-swap (b,c) */
  if( b > c ) { t = b; b = c; c = t; }

  /* Pack sorted values: a(min) in low bits, d(max) in high */
  return (d << 24) | ((c & 0xFF) << 16) | ((b & 0xFF) << 8) | (a & 0xFF);
}

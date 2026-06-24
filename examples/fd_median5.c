/* fd_median5 — median of 5 values using sorting network.
   Tests: deep if/else chains + multiple variable swaps.
   Real-world: median filter for image/signal denoising. */

typedef unsigned int uint;

uint
fd_median5( uint a, uint b, uint c, uint d, uint e ) {
  /* Optimal 5-element sorting network (9 comparisons) */
  uint t;

  /* Sort pairs */
  if( a > b ) { t = a; a = b; b = t; }
  if( c > d ) { t = c; c = d; d = t; }

  /* Sort low pair vs high pair */
  if( a > c ) { t = a; a = c; c = t; t = b; b = d; d = t; }

  /* Insert e */
  if( b > d ) { t = b; b = d; d = t; }
  if( c > e ) { t = c; c = e; e = t; }
  if( b > c ) { t = b; b = c; c = t; }
  if( c > e ) { t = c; c = e; e = t; }

  /* c is now the median */
  return c;
}

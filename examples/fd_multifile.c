/* fd_multifile — uses functions from fd_helpers.c.
   Compile: c2v examples/fd_helpers.c examples/fd_multifile.c -f fd_blend */

typedef unsigned int uint;

uint fd_sat_add( uint a, uint b );
uint fd_sat_sub( uint a, uint b );

uint
fd_blend( uint a, uint b, uint alpha ) {
  /* alpha blend: result = a * alpha/256 + b * (256-alpha)/256 */
  uint wa = fd_sat_add( (a >> 8) * alpha, (a & 0xFF) * alpha );
  uint wb = fd_sat_add( (b >> 8) * (256 - alpha), (b & 0xFF) * (256 - alpha) );
  return fd_sat_add( wa >> 8, wb >> 8 );
}

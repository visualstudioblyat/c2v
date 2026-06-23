/* fd_bayer_demosaic — Bayer pattern demosaicing (2x2 block).
   Tests: array access + arithmetic + if/else.
   Real-world: camera ISP on FPGA (image sensor processing). */

typedef unsigned int uint;

uint
fd_demosaic_pixel( uint p00, uint p01, uint p10, uint p11,
                   uint row, uint col ) {
  /* Bayer pattern RGGB:
     p00=R  p01=G
     p10=G  p11=B  */

  uint r;
  uint g;
  uint b;
  uint pos = ((row & 1) << 1) | (col & 1);

  if( pos == 0 ) {
    /* Red position: R=p00, G=avg(p01,p10), B=p11 */
    r = p00;
    g = (p01 + p10) >> 1;
    b = p11;
  } else if( pos == 1 ) {
    /* Green on red row: R=p00, G=p01, B=avg(p10+extra) */
    r = p00;
    g = p01;
    b = (p10 + p11) >> 1;
  } else if( pos == 2 ) {
    /* Green on blue row: R=avg(p00+extra), G=p10, B=p11 */
    r = (p00 + p01) >> 1;
    g = p10;
    b = p11;
  } else {
    /* Blue position: R=p00, G=avg(p01,p10), B=p11 */
    r = p00;
    g = (p01 + p10) >> 1;
    b = p11;
  }

  /* Clamp to 8 bits */
  if( r > 255 ) r = 255;
  if( g > 255 ) g = 255;
  if( b > 255 ) b = 255;

  return (r << 16) | (g << 8) | b;
}

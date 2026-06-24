/* fd_sobel — Sobel edge detection on 3x3 pixel window.
   Tests: array + multiply-accumulate + absolute value.
   Real-world: image processing, computer vision on FPGA. */

typedef unsigned int uint;

uint
fd_sobel_magnitude( uint p00, uint p01, uint p02,
                    uint p10, uint p11, uint p12,
                    uint p20, uint p21, uint p22 ) {
  /* Gx kernel: [-1 0 1; -2 0 2; -1 0 1] */
  uint gx = p02 + 2 * p12 + p22;
  uint gx_neg = p00 + 2 * p10 + p20;
  uint gx_abs;
  if( gx >= gx_neg ) {
    gx_abs = gx - gx_neg;
  } else {
    gx_abs = gx_neg - gx;
  }

  /* Gy kernel: [-1 -2 -1; 0 0 0; 1 2 1] */
  uint gy = p20 + 2 * p21 + p22;
  uint gy_neg = p00 + 2 * p01 + p02;
  uint gy_abs;
  if( gy >= gy_neg ) {
    gy_abs = gy - gy_neg;
  } else {
    gy_abs = gy_neg - gy;
  }

  /* Approximate magnitude: |Gx| + |Gy| */
  uint mag = gx_abs + gy_abs;

  /* Threshold */
  if( mag > 255 ) mag = 255;

  return mag;
}

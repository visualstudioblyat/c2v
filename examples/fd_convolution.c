/* fd_convolution — 3x3 convolution kernel.
   Tests: nested loop + array + multiply-accumulate.
   Real-world: image processing, edge detection. */

typedef unsigned int uint;

uint
fd_conv3x3( uint p00, uint p01, uint p02,
            uint p10, uint p11, uint p12,
            uint p20, uint p21, uint p22,
            uint k00, uint k01, uint k02,
            uint k10, uint k11, uint k12,
            uint k20, uint k21, uint k22 ) {
  uint pixels[9] = { p00, p01, p02, p10, p11, p12, p20, p21, p22 };
  uint kernel[9] = { k00, k01, k02, k10, k11, k12, k20, k21, k22 };

  uint acc = 0;
  for( int i = 0; i < 9; i++ ) {
    acc = acc + pixels[i] * kernel[i];
  }
  return acc >> 8;
}

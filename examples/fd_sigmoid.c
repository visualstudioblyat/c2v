/* fd_sigmoid — piecewise linear sigmoid approximation.
   Tests: chained if/else + multiply + shift.
   Real-world: ML inference activation function on FPGA. */

typedef unsigned int uint;

uint
fd_sigmoid_approx( uint x ) {
  /* Fixed-point Q8.8: input and output in range [0, 65535]
     Approximation of sigmoid(x-128) * 256 */
  uint input = x & 0xFFFF;

  uint result;
  if( input < 64 ) {
    result = 0;
  } else if( input < 96 ) {
    result = (input - 64) * 2;
  } else if( input < 112 ) {
    result = 64 + (input - 96) * 4;
  } else if( input < 128 ) {
    result = 128 + (input - 112) * 4;
  } else if( input < 144 ) {
    result = 192 + (input - 128) * 4;
  } else if( input < 160 ) {
    result = 256 - (160 - input) * 4;
  } else if( input < 192 ) {
    result = 256 - (192 - input) * 2;
  } else {
    result = 256;
  }

  return result;
}

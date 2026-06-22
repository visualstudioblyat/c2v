/* fd_relu — ReLU + quantized linear layer (4 neurons).
   Tests: loop + if + multiply-accumulate.
   Real-world: ML inference on FPGA. */

typedef unsigned int uint;

uint
fd_relu_linear4( uint x0, uint x1, uint x2, uint x3,
                 uint w00, uint w01, uint w02, uint w03,
                 uint w10, uint w11, uint w12, uint w13,
                 uint bias0, uint bias1 ) {
  uint x[4] = { x0, x1, x2, x3 };
  uint w0[4] = { w00, w01, w02, w03 };
  uint w1[4] = { w10, w11, w12, w13 };

  /* Neuron 0: dot product + bias + ReLU */
  uint acc0 = bias0;
  for( int i = 0; i < 4; i++ ) {
    acc0 = acc0 + x[i] * w0[i];
  }
  uint y0 = acc0 >> 8;
  if( y0 > 255 ) y0 = 255;

  /* Neuron 1: dot product + bias + ReLU */
  uint acc1 = bias1;
  for( int i = 0; i < 4; i++ ) {
    acc1 = acc1 + x[i] * w1[i];
  }
  uint y1 = acc1 >> 8;
  if( y1 > 255 ) y1 = 255;

  return (y0 << 8) | y1;
}

/* fd_iir_biquad — IIR biquad filter, 2nd order.
   Tests: multiply-accumulate with feedback, array state.
   Real-world: audio DSP, control loops, sensor filtering. */

typedef unsigned int uint;

uint
fd_iir_biquad( uint x_in,
               uint b0, uint b1, uint b2,
               uint a1, uint a2,
               uint x1, uint x2,
               uint y1, uint y2 ) {
  /* Direct Form I: y[n] = b0*x[n] + b1*x[n-1] + b2*x[n-2] - a1*y[n-1] - a2*y[n-2] */
  uint y = b0 * x_in + b1 * x1 + b2 * x2;
  uint feedback = a1 * y1 + a2 * y2;
  uint result;
  if( y >= feedback ) {
    result = y - feedback;
  } else {
    result = 0;
  }
  return result >> 8;
}

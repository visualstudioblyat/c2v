/* fd_moving_avg — 4-sample moving average filter.
   Tests: array shift + accumulation.
   Real-world: sensor smoothing, ADC filtering, display pipeline. */

typedef unsigned int uint;

uint
fd_moving_avg4( uint s0, uint s1, uint s2, uint s3 ) {
  uint sum = s0 + s1 + s2 + s3;
  uint avg = sum >> 2;
  return avg;
}

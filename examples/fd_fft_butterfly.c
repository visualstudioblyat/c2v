/* fd_fft_butterfly — radix-2 FFT butterfly (fixed-point).
   Tests: multiply + add/sub + struct.
   Real-world: SDR, spectrum analysis, OFDM. */

typedef unsigned int uint;

typedef struct {
  uint re;
  uint im;
} complex_t;

uint
fd_butterfly( uint a_re, uint a_im,
              uint b_re, uint b_im,
              uint w_re, uint w_im ) {
  /* Twiddle factor multiplication: t = b * w */
  uint t_re = ((b_re * w_re) >> 15) - ((b_im * w_im) >> 15);
  uint t_im = ((b_re * w_im) >> 15) + ((b_im * w_re) >> 15);

  /* Butterfly: out_a = a + t, out_b = a - t */
  uint out_a_re = a_re + t_re;
  uint out_a_im = a_im + t_im;
  uint out_b_re = a_re - t_re;
  uint out_b_im = a_im - t_im;

  /* Pack first output (16-bit each) */
  return ((out_a_re & 0xFFFF) << 16) | (out_a_im & 0xFFFF);
}

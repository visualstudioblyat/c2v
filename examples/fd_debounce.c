/* fd_debounce — button debounce with shift register.
   Tests: shift + threshold compare + state tracking.
   Real-world: GPIO input filtering, sensor noise rejection. */

typedef unsigned int uint;

uint
fd_debounce( uint shift_reg, uint new_sample, uint threshold ) {
  /* Shift in new sample */
  uint sr = (shift_reg << 1) | (new_sample & 1);
  uint mask = (1 << threshold) - 1;
  uint window = sr & mask;

  /* All 1s in window → stable high */
  uint stable_high = (window == mask) ? 1 : 0;
  /* All 0s in window → stable low */
  uint stable_low = (window == 0) ? 1 : 0;

  return (stable_high << 31) | (stable_low << 30) | (sr & 0x3FFFFFFF);
}

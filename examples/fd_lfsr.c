/* fd_lfsr — linear feedback shift register.
   Tests: loop + conditional XOR + bit manipulation.
   Real-world: PRNG, scrambling, BIST. */

typedef unsigned int uint;

uint
fd_lfsr_advance( uint state, uint taps, uint steps ) {
  uint s = state;
  for( int i = 0; i < 16; i++ ) {
    uint feedback = s & 1;
    s >>= 1;
    if( feedback ) {
      s ^= taps;
    }
  }
  return s;
}

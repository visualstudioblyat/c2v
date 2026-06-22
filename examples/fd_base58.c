/* fd_base58 — Base58 digit extraction (one digit per call).
   Tests: division + modulo + lookup table.
   Real-world: Solana address encoding. */

typedef unsigned int uint;

uint
fd_base58_digit( uint value, uint digit_pos ) {
  uint alphabet[16] = {
    0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38,
    0x39, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47
  };

  uint v = value;
  for( int i = 0; i < 4; i++ ) {
    if( i < digit_pos ) {
      v = v >> 4;
    }
  }
  uint idx = v & 0xF;
  return alphabet[idx];
}

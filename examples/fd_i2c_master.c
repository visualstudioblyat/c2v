/* fd_i2c_master — I2C master byte transfer state logic.
   Tests: struct + nested if/else + bit shifting.
   Real-world: sensor reading, EEPROM programming. */

typedef unsigned int uint;

typedef struct {
  uint sda_out;
  uint scl_out;
  uint bit_idx;
  uint phase;
  uint ack_bit;
} i2c_state_t;

uint
fd_i2c_shift_byte( uint tx_byte, uint rx_sda,
                   uint bit_idx, uint phase ) {
  i2c_state_t s;
  s.bit_idx = bit_idx;
  s.phase = phase;
  s.ack_bit = 0;

  uint next_bit_idx = s.bit_idx;
  uint next_phase = s.phase;
  uint sda = 1;
  uint scl = 0;

  if( s.phase == 0 ) {
    sda = (tx_byte >> (7 - s.bit_idx)) & 1;
    scl = 0;
    next_phase = 1;
  } else if( s.phase == 1 ) {
    sda = (tx_byte >> (7 - s.bit_idx)) & 1;
    scl = 1;
    next_phase = 2;
  } else if( s.phase == 2 ) {
    scl = 1;
    s.ack_bit = rx_sda;
    next_phase = 3;
  } else {
    scl = 0;
    next_phase = 0;
    if( s.bit_idx < 7 ) {
      next_bit_idx = s.bit_idx + 1;
    } else {
      next_bit_idx = 0;
    }
  }

  return (sda << 24) | (scl << 16) | (next_bit_idx << 8) |
         (next_phase << 4) | s.ack_bit;
}

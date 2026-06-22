/* fd_spi_master — SPI master byte shift.
   Tests: loop + bit shifting + struct state.
   Real-world: sensor interface, flash programming. */

typedef unsigned int uint;

typedef struct {
  uint mosi_data;
  uint miso_data;
  uint bit_count;
  uint clock_pol;
} spi_state_t;

uint
fd_spi_shift_byte( uint tx_byte, uint rx_bits,
                   uint cpol, uint bit_count ) {
  spi_state_t s;
  s.mosi_data = tx_byte;
  s.miso_data = 0;
  s.clock_pol = cpol;
  s.bit_count = bit_count;

  for( int i = 0; i < 8; i++ ) {
    /* Shift out MSB first */
    uint mosi_bit = (s.mosi_data >> (7 - i)) & 1;
    /* Shift in MISO */
    uint miso_bit = (rx_bits >> (7 - i)) & 1;
    s.miso_data = s.miso_data | (miso_bit << i);
  }

  return (s.mosi_data << 8) | s.miso_data;
}

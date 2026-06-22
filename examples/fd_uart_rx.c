/* fd_uart_rx — UART byte receiver state machine.
   Tests: struct state + if/else chains + bit extraction.
   Real-world: serial communication on FPGA. */

typedef unsigned int uint;

typedef struct {
  uint shift_reg;
  uint bit_count;
  uint baud_counter;
  uint data_ready;
  uint rx_byte;
} uart_rx_t;

uint
fd_uart_rx_tick( uint rx_pin, uint baud_div,
                 uint shift_reg, uint bit_count,
                 uint baud_counter ) {
  uart_rx_t s;
  s.shift_reg = shift_reg;
  s.bit_count = bit_count;
  s.baud_counter = baud_counter;
  s.data_ready = 0;
  s.rx_byte = 0;

  if( s.baud_counter > 0 ) {
    s.baud_counter = s.baud_counter - 1;
  } else {
    s.baud_counter = baud_div;
    if( s.bit_count > 0 ) {
      s.shift_reg = (s.shift_reg >> 1) | ((rx_pin & 1) << 7);
      s.bit_count = s.bit_count - 1;
      if( s.bit_count == 0 ) {
        s.rx_byte = s.shift_reg & 0xFF;
        s.data_ready = 1;
      }
    } else {
      if( rx_pin == 0 ) {
        s.bit_count = 8;
        s.baud_counter = baud_div >> 1;
      }
    }
  }

  return (s.data_ready << 24) | (s.rx_byte << 16) |
         (s.bit_count << 8) | (s.baud_counter & 0xFF);
}

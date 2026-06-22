/* fd_huffman — Huffman decoder (fixed table, 4-bit symbols).
   Tests: array lookup + if/else chain + bit extraction.
   Real-world: data compression on FPGA. */

typedef unsigned int uint;

uint
fd_huffman_decode( uint bitstream, uint offset ) {
  /* Fixed Huffman table for 4-bit symbols:
     0    → sym 0  (1 bit)
     10   → sym 1  (2 bits)
     110  → sym 2  (3 bits)
     1110 → sym 3  (4 bits)
     1111 → sym 4  (4 bits) */

  uint bits = (bitstream >> offset) & 0xF;
  uint symbol;
  uint length;

  if( (bits & 1) == 0 ) {
    symbol = 0;
    length = 1;
  } else if( (bits & 3) == 1 ) {
    symbol = 1;
    length = 2;
  } else if( (bits & 7) == 3 ) {
    symbol = 2;
    length = 3;
  } else if( (bits & 15) == 7 ) {
    symbol = 3;
    length = 4;
  } else {
    symbol = 4;
    length = 4;
  }

  return (symbol << 8) | length;
}

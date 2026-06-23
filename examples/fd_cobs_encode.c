/* fd_cobs_encode — COBS (Consistent Overhead Byte Stuffing) encoder step.
   Tests: loop + if/else + array write.
   Real-world: serial protocol framing (embedded FPGA comms). */

typedef unsigned int uint;

uint
fd_cobs_encode_block( uint b0, uint b1, uint b2, uint b3,
                      uint b4, uint b5, uint b6, uint b7 ) {
  uint data[8] = { b0, b1, b2, b3, b4, b5, b6, b7 };
  uint encoded = 0;
  uint code = 1;
  uint code_idx = 0;

  for( int i = 0; i < 8; i++ ) {
    uint byte = data[i] & 0xFF;
    if( byte == 0 ) {
      encoded = encoded | (code << (code_idx * 4));
      code = 1;
      code_idx = code_idx + 1;
    } else {
      code = code + 1;
    }
  }

  /* Final code byte */
  encoded = encoded | (code << (code_idx * 4));

  return encoded;
}

/* fd_crc8 — CRC-8 computation via loop + conditional XOR.
   Real-world use: protocol checksums on FPGA. */

typedef unsigned int uint;
typedef unsigned char uchar;

uint
fd_crc8( uint data, uint len ) {
  uint crc = 0xFF;
  for( int i = 0; i < 8; i++ ) {
    if( (crc ^ (data >> i)) & 1 ) {
      crc = (crc >> 1) ^ 0x8C;
    } else {
      crc = crc >> 1;
    }
  }
  return crc & 0xFF;
}

typedef unsigned char  uint8;
typedef unsigned int   uint;

uint
fd_byte_reverse( uint x ) {
  uint8 b0 = x & 0xFF;
  uint8 b1 = (x >> 8) & 0xFF;
  uint8 b2 = (x >> 16) & 0xFF;
  uint8 b3 = (x >> 24) & 0xFF;
  return ((uint)b0 << 24) | ((uint)b1 << 16) | ((uint)b2 << 8) | (uint)b3;
}

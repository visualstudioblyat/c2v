typedef unsigned char  uint8;
typedef unsigned short uint16;
typedef unsigned int   uint;

uint
fd_rgb565_pack( uint8 r, uint8 g, uint8 b ) {
  uint16 r5 = (r >> 3) & 0x1F;
  uint16 g6 = (g >> 2) & 0x3F;
  uint16 b5 = (b >> 3) & 0x1F;
  return (r5 << 11) | (g6 << 5) | b5;
}

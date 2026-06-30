typedef unsigned char  uint8;
typedef unsigned int   uint;

uint
fd_alpha_blend( uint fg, uint bg, uint8 alpha ) {
  uint8 fg_r = (fg >> 16) & 0xFF;
  uint8 fg_g = (fg >> 8) & 0xFF;
  uint8 fg_b = fg & 0xFF;
  uint8 bg_r = (bg >> 16) & 0xFF;
  uint8 bg_g = (bg >> 8) & 0xFF;
  uint8 bg_b = bg & 0xFF;

  uint8 inv = 255 - alpha;
  uint r = (fg_r * alpha + bg_r * inv) >> 8;
  uint g = (fg_g * alpha + bg_g * inv) >> 8;
  uint b = (fg_b * alpha + bg_b * inv) >> 8;

  return (r << 16) | (g << 8) | b;
}

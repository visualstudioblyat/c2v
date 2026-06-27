typedef unsigned int uint;

uint
fd_rgb_to_y( uint r, uint g, uint b ) {
  return (66 * r + 129 * g + 25 * b + 128) >> 8;
}

uint
fd_rgb_to_yuv( uint r, uint g, uint b ) {
  uint y  = (66 * r + 129 * g + 25 * b + 128) >> 8;
  uint u  = ((-38 * r + (-74) * g + 112 * b + 128) >> 8) + 128;
  uint v  = ((112 * r + (-94) * g + (-18) * b + 128) >> 8) + 128;
  return (y & 0xFF) | ((u & 0xFF) << 8) | ((v & 0xFF) << 16);
}

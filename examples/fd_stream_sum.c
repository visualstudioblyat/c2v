typedef unsigned int uint;

uint
fd_element_process( uint x ) {
  uint squared = x * x;
  uint masked  = squared & 0xFFFF;
  uint shifted = masked >> 2;
  return shifted + x;
}

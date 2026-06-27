typedef unsigned int uint;

uint
fd_clz32( uint x ) {
  uint n16 = (x >> 16) ? 0 : 16;
  uint x16 = (x >> 16) ? (x >> 16) : x;
  uint n8  = (x16 >> 8) ? 0 : 8;
  uint x8  = (x16 >> 8) ? (x16 >> 8) : x16;
  uint n4  = (x8 >> 4) ? 0 : 4;
  uint x4  = (x8 >> 4) ? (x8 >> 4) : x8;
  uint n2  = (x4 >> 2) ? 0 : 2;
  uint x2  = (x4 >> 2) ? (x4 >> 2) : x4;
  uint n1  = (x2 >> 1) ? 0 : 1;
  uint x1  = (x2 >> 1) ? (x2 >> 1) : x2;
  uint n0  = 1 - (x1 & 1);
  return n16 + n8 + n4 + n2 + n1 + n0;
}

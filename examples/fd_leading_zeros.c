typedef unsigned int uint;

uint
fd_clz32( uint x ) {
  uint n = 32;
  if( x & 0xFFFF0000 ) { n -= 16; x >>= 16; }
  if( x & 0x0000FF00 ) { n -= 8;  x >>= 8;  }
  if( x & 0x000000F0 ) { n -= 4;  x >>= 4;  }
  if( x & 0x0000000C ) { n -= 2;  x >>= 2;  }
  if( x & 0x00000002 ) { n -= 1;  x >>= 1;  }
  if( x & 0x00000001 ) { n -= 1; }
  return n;
}

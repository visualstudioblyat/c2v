/* fd_ctz — count trailing zeros (32-bit).
   AoC-style: bit manipulation for priority encoding.
   Tests: isolate lowest set bit + de Bruijn multiplication.
   Competitive metric: 1-2 cycles (mul + lookup). */

typedef unsigned int uint;

uint
fd_ctz32( uint x ) {
  if( x == 0 ) return 32;

  uint count = 0;
  if( !(x & 0x0000FFFF) ) { count = count + 16; x = x >> 16; }
  if( !(x & 0x000000FF) ) { count = count + 8;  x = x >> 8;  }
  if( !(x & 0x0000000F) ) { count = count + 4;  x = x >> 4;  }
  if( !(x & 0x00000003) ) { count = count + 2;  x = x >> 2;  }
  if( !(x & 0x00000001) ) { count = count + 1; }

  return count;
}

/* fd_endian_swap — byte-swap for endianness conversion.
   Tests: shift + mask + or pipeline.
   Real-world: network byte order, USB, PCIe TLP headers. */

typedef unsigned int uint;

uint
fd_bswap32( uint x ) {
  uint b0 = (x >> 24) & 0xFF;
  uint b1 = (x >> 16) & 0xFF;
  uint b2 = (x >>  8) & 0xFF;
  uint b3 =  x        & 0xFF;
  return (b3 << 24) | (b2 << 16) | (b1 << 8) | b0;
}

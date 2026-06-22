/* fd_reed_solomon — GF(256) multiply for Reed-Solomon codes.
   Tests: lookup table + XOR + loop.
   Real-world: storage ECC, QR codes, Solana erasure coding. */

typedef unsigned int uint;

uint
fd_gf256_mul( uint a, uint b ) {
  /* Galois field multiply in GF(2^8) with polynomial 0x11b */
  uint result = 0;
  uint aa = a & 0xFF;
  uint bb = b & 0xFF;

  for( int i = 0; i < 8; i++ ) {
    if( bb & 1 ) {
      result = result ^ aa;
    }
    uint hi_bit = aa & 0x80;
    aa = (aa << 1) & 0xFF;
    if( hi_bit ) {
      aa = aa ^ 0x1b;
    }
    bb = bb >> 1;
  }
  return result & 0xFF;
}

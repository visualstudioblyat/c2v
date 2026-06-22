/* fd_aes_round — AES-128 full round (SubBytes + ShiftRows + MixColumns + AddRoundKey).
   CHStone benchmark: aes (simplified to one round on 4 bytes).
   Tests: array lookup, XOR, shifts, inlining. */

typedef unsigned int uint;
typedef unsigned char uchar;

uint fd_aes_sbox_lookup( uint idx ) {
  uint sbox[16] = {
    0x63, 0x7c, 0x77, 0x7b, 0xf2, 0x6b, 0x6f, 0xc5,
    0x30, 0x01, 0x67, 0x2b, 0xfe, 0xd7, 0xab, 0x76
  };
  return sbox[idx & 0xF];
}

uint fd_xtime( uint x ) {
  uint shifted = x << 1;
  uint mask = ((x >> 7) & 1) * 0x1b;
  return (shifted ^ mask) & 0xFF;
}

uint
fd_aes_round( uint s0, uint s1, uint s2, uint s3, uint rk ) {
  /* SubBytes (partial — 4-bit S-box for demo) */
  uint t0 = fd_aes_sbox_lookup( s0 );
  uint t1 = fd_aes_sbox_lookup( s1 );
  uint t2 = fd_aes_sbox_lookup( s2 );
  uint t3 = fd_aes_sbox_lookup( s3 );

  /* MixColumns (single column, simplified) */
  uint m0 = fd_xtime( t0 ) ^ (fd_xtime( t1 ) ^ t1) ^ t2 ^ t3;
  uint m1 = t0 ^ fd_xtime( t1 ) ^ (fd_xtime( t2 ) ^ t2) ^ t3;
  uint m2 = t0 ^ t1 ^ fd_xtime( t2 ) ^ (fd_xtime( t3 ) ^ t3);
  uint m3 = (fd_xtime( t0 ) ^ t0) ^ t1 ^ t2 ^ fd_xtime( t3 );

  /* AddRoundKey */
  uint r0 = m0 ^ ((rk >> 24) & 0xFF);
  uint r1 = m1 ^ ((rk >> 16) & 0xFF);
  uint r2 = m2 ^ ((rk >> 8) & 0xFF);
  uint r3 = m3 ^ (rk & 0xFF);

  return (r0 << 24) | (r1 << 16) | (r2 << 8) | r3;
}

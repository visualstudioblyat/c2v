/* fd_aes_sbox — AES SubBytes via lookup table.
   16-byte state, each byte goes through the S-box.
   Tests loop unrolling + array read pattern. */

typedef unsigned char uchar;
typedef unsigned int  uint;

uint
fd_aes_sbox_round( uint b0,  uint b1,  uint b2,  uint b3,
                   uint b4,  uint b5,  uint b6,  uint b7,
                   uint b8,  uint b9,  uint b10, uint b11,
                   uint b12, uint b13, uint b14, uint b15 ) {
  /* Partial S-box (first 16 entries) */
  uint sbox[16] = { 0x63, 0x7c, 0x77, 0x7b,
                    0xf2, 0x6b, 0x6f, 0xc5,
                    0x30, 0x01, 0x67, 0x2b,
                    0xfe, 0xd7, 0xab, 0x76 };

  uint state[16] = { b0, b1, b2, b3, b4, b5, b6, b7,
                     b8, b9, b10, b11, b12, b13, b14, b15 };

  uint result = 0;
  for( int i = 0; i < 16; i++ ) {
    uint idx = state[i] & 15;
    uint sub = sbox[idx];
    result = result ^ (sub << (i & 3) * 8);
  }
  return result;
}

/* fd_blowfish_round — Blowfish Feistel round (simplified).
   CHStone benchmark: blowfish (single round).
   Tests: array lookup + XOR + split/combine. */

typedef unsigned int uint;

uint
fd_blowfish_f( uint x,
               uint s0_0, uint s0_1, uint s0_2, uint s0_3,
               uint s1_0, uint s1_1, uint s1_2, uint s1_3,
               uint s2_0, uint s2_1, uint s2_2, uint s2_3,
               uint s3_0, uint s3_1, uint s3_2, uint s3_3 ) {
  /* S-boxes (4x4 entries for demo — real Blowfish uses 4x256) */
  uint sbox0[4] = { s0_0, s0_1, s0_2, s0_3 };
  uint sbox1[4] = { s1_0, s1_1, s1_2, s1_3 };
  uint sbox2[4] = { s2_0, s2_1, s2_2, s2_3 };
  uint sbox3[4] = { s3_0, s3_1, s3_2, s3_3 };

  uint b0 = (x >> 6) & 3;
  uint b1 = (x >> 4) & 3;
  uint b2 = (x >> 2) & 3;
  uint b3 = x & 3;

  uint y = ((sbox0[b0] + sbox1[b1]) ^ sbox2[b2]) + sbox3[b3];
  return y;
}

uint
fd_blowfish_round( uint xl, uint xr, uint p_key ) {
  uint f_out = fd_blowfish_f( xl,
    0xd1310ba6, 0x98dfb5ac, 0x2ffd72db, 0xd01adfb7,
    0xb8e1afed, 0x6a267e96, 0xba7c9045, 0xf12c7f99,
    0x24a19947, 0xb3916cf7, 0x0801f2e2, 0x858efc16,
    0x636920d8, 0x71574e69, 0xa458fea3, 0xf4933d7e );
  uint new_xr = xl ^ p_key;
  uint new_xl = f_out ^ xr;
  return (new_xl << 16) | (new_xr & 0xFFFF);
}

typedef unsigned int uint;

uint
fd_xtime( uint a ) {
  uint shifted = (a << 1) & 0xFF;
  uint mask = ((a >> 7) & 1) ? 0x1B : 0;
  return shifted ^ mask;
}

uint
fd_aes_mix_column( uint s0, uint s1, uint s2, uint s3 ) {
  uint t0 = fd_xtime(s0 ^ s1) ^ s1 ^ s2 ^ s3;
  uint t1 = fd_xtime(s1 ^ s2) ^ s0 ^ s2 ^ s3;
  uint t2 = fd_xtime(s2 ^ s3) ^ s0 ^ s1 ^ s3;
  uint t3 = fd_xtime(s3 ^ s0) ^ s0 ^ s1 ^ s2;
  return (t0 & 0xFF) | ((t1 & 0xFF) << 8) | ((t2 & 0xFF) << 16) | ((t3 & 0xFF) << 24);
}

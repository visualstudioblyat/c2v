typedef unsigned int uint;

uint
fd_rs_gf_add( uint a, uint b ) {
  return (a ^ b) & 0xFF;
}

uint
fd_rs_syndrome( uint r0, uint r1, uint r2, uint r3,
                uint g0, uint g1, uint g2, uint g3 ) {
  uint s0 = r0 ^ g0;
  uint s1 = r1 ^ g1;
  uint s2 = r2 ^ g2;
  uint s3 = r3 ^ g3;
  uint syndrome = (s0 & 0xFF) | ((s1 & 0xFF) << 8) | ((s2 & 0xFF) << 16) | ((s3 & 0xFF) << 24);
  uint is_error = (syndrome != 0) ? 1 : 0;
  return (is_error << 31) | syndrome;
}

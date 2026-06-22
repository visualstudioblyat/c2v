/* fd_sha256_compress — SHA-256 compression function (4 rounds).
   Tests: loop + inlining + array + bitwise + struct.
   Real-world: blockchain, TLS, certificate validation. */

typedef unsigned int uint;

uint fd_sha256_ch( uint e, uint f, uint g ) {
  return (e & f) ^ ((~e) & g);
}

uint fd_sha256_maj( uint a, uint b, uint c ) {
  return (a & b) ^ (a & c) ^ (b & c);
}

uint fd_sha256_sigma0( uint a ) {
  return ((a >> 2) | (a << 30)) ^ ((a >> 13) | (a << 19)) ^ ((a >> 22) | (a << 10));
}

uint fd_sha256_sigma1( uint e ) {
  return ((e >> 6) | (e << 26)) ^ ((e >> 11) | (e << 21)) ^ ((e >> 25) | (e << 7));
}

uint
fd_sha256_compress4( uint a, uint b, uint c, uint d,
                     uint e, uint f, uint g, uint h,
                     uint w0, uint w1, uint w2, uint w3,
                     uint k0, uint k1, uint k2, uint k3 ) {
  uint w[4] = { w0, w1, w2, w3 };
  uint k[4] = { k0, k1, k2, k3 };

  uint state_a = a;
  uint state_b = b;
  uint state_c = c;
  uint state_d = d;
  uint state_e = e;
  uint state_f = f;
  uint state_g = g;
  uint state_h = h;

  for( int i = 0; i < 4; i++ ) {
    uint s1 = fd_sha256_sigma1( state_e );
    uint ch = fd_sha256_ch( state_e, state_f, state_g );
    uint t1 = state_h + s1 + ch + k[i] + w[i];
    uint s0 = fd_sha256_sigma0( state_a );
    uint maj = fd_sha256_maj( state_a, state_b, state_c );
    uint t2 = s0 + maj;

    state_h = state_g;
    state_g = state_f;
    state_f = state_e;
    state_e = state_d + t1;
    state_d = state_c;
    state_c = state_b;
    state_b = state_a;
    state_a = t1 + t2;
  }

  return state_a ^ state_e;
}

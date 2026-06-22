/* fd_chacha_qr — ChaCha20 quarter round.
   Tests: rotate + add + XOR (the core primitive).
   Real-world: TLS 1.3 encryption on FPGA. */

typedef unsigned int uint;

uint fd_rotl( uint x, uint n ) {
  return (x << n) | (x >> (32 - n));
}

uint
fd_chacha_quarter_round( uint a, uint b, uint c, uint d ) {
  a = a + b; d = fd_rotl( d ^ a, 16 );
  c = c + d; b = fd_rotl( b ^ c, 12 );
  a = a + b; d = fd_rotl( d ^ a, 8 );
  c = c + d; b = fd_rotl( b ^ c, 7 );
  return a ^ b ^ c ^ d;
}

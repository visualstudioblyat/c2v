/* fd_blake3_round — BLAKE3 quarter-round + diagonal round.
   Tests: rotate + add + XOR (like ChaCha but different constants).
   Real-world: fast hashing, Merkle proofs, Solana PoH. */

typedef unsigned int uint;

uint fd_rotr( uint x, uint n ) {
  return (x >> n) | (x << (32 - n));
}

uint
fd_blake3_g( uint a, uint b, uint c, uint d, uint mx, uint my ) {
  a = a + b + mx;
  d = fd_rotr( d ^ a, 16 );
  c = c + d;
  b = fd_rotr( b ^ c, 12 );
  a = a + b + my;
  d = fd_rotr( d ^ a, 8 );
  c = c + d;
  b = fd_rotr( b ^ c, 7 );
  return a ^ b ^ c ^ d;
}

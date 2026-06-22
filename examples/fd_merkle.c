/* fd_merkle — Merkle tree hash combine step.
   Tests: hash function + comparison.
   Real-world: blockchain state proofs, certificate chains. */

typedef unsigned int uint;

uint fd_merkle_hash( uint a, uint b ) {
  /* Simplified hash combine: FNV-1a-like */
  uint h = 0x811c9dc5;
  h = h ^ a;
  h = h * 0x01000193;
  h = h ^ b;
  h = h * 0x01000193;
  h = h ^ (a >> 16);
  h = h * 0x01000193;
  h = h ^ (b >> 16);
  h = h * 0x01000193;
  return h;
}

uint
fd_merkle_combine( uint left_hash, uint right_hash,
                   uint expected_root, uint depth ) {
  /* Canonical ordering: smaller hash first */
  uint first;
  uint second;
  if( left_hash <= right_hash ) {
    first  = left_hash;
    second = right_hash;
  } else {
    first  = right_hash;
    second = left_hash;
  }

  uint combined = fd_merkle_hash( first, second );

  /* Check if combined matches expected root */
  uint valid;
  if( combined == expected_root ) {
    valid = 1;
  } else {
    valid = 0;
  }

  return (valid << 31) | (combined & 0x7FFFFFFF);
}

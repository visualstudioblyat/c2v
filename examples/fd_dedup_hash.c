/* fd_dedup_hash — transaction dedup via hash comparison.
   Tests: multi-field hash + equality check.
   Real-world: Firedancer dedup tile. */

typedef unsigned int uint;

uint fd_hash_combine( uint h, uint val ) {
  h ^= val;
  h = h * 0x01000193;
  return h;
}

uint
fd_dedup_check( uint sig0, uint sig1, uint sig2, uint sig3,
                uint cache0, uint cache1, uint cache2, uint cache3 ) {
  /* Hash the signature */
  uint h = 0x811c9dc5;
  h = fd_hash_combine( h, sig0 );
  h = fd_hash_combine( h, sig1 );
  h = fd_hash_combine( h, sig2 );
  h = fd_hash_combine( h, sig3 );

  /* Hash the cached signature */
  uint c = 0x811c9dc5;
  c = fd_hash_combine( c, cache0 );
  c = fd_hash_combine( c, cache1 );
  c = fd_hash_combine( c, cache2 );
  c = fd_hash_combine( c, cache3 );

  /* Compare: 1 if duplicate, 0 if unique */
  uint is_dup;
  if( h == c ) {
    is_dup = 1;
  } else {
    is_dup = 0;
  }
  return (is_dup << 31) | (h & 0x7FFFFFFF);
}

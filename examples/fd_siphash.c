/* fd_siphash — SipHash-2-4 compression round.
   Tests: rotate + add + XOR on 64-bit values.
   Real-world: hash table defense, network packet hashing. */

typedef unsigned long ulong;
typedef unsigned int  uint;

ulong fd_rotl64( ulong x, uint n ) {
  return (x << n) | (x >> (64 - n));
}

ulong
fd_siphash_round( ulong v0, ulong v1, ulong v2, ulong v3 ) {
  v0 = v0 + v1;
  v1 = fd_rotl64( v1, 13 );
  v1 = v1 ^ v0;
  v0 = fd_rotl64( v0, 32 );

  v2 = v2 + v3;
  v3 = fd_rotl64( v3, 16 );
  v3 = v3 ^ v2;

  v0 = v0 + v3;
  v3 = fd_rotl64( v3, 21 );
  v3 = v3 ^ v0;

  v2 = v2 + v1;
  v1 = fd_rotl64( v1, 17 );
  v1 = v1 ^ v2;
  v2 = fd_rotl64( v2, 32 );

  return v0 ^ v1 ^ v2 ^ v3;
}

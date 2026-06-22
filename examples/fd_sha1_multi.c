/* fd_sha1_multi — SHA-1 with round-dependent f/k selection.
   Tests: if/else chain with input-dependent mux + inlining.
   This was failing with the old expression-based reference model. */

typedef unsigned int uint;

uint fd_sha1_rotl( uint x, uint n ) {
  return (x << n) | (x >> (32 - n));
}

uint
fd_sha1_multi( uint a, uint b, uint c, uint d, uint e,
               uint w, uint round_type ) {
  /* round_type: 0=Ch, 1=Parity, 2=Maj, 3=Parity */
  uint f;
  uint k;
  uint rt = round_type & 3;

  if( rt == 0 ) {
    f = (b & c) | ((~b) & d);
    k = 0x5A827999;
  } else if( rt == 1 ) {
    f = b ^ c ^ d;
    k = 0x6ED9EBA1;
  } else if( rt == 2 ) {
    f = (b & c) | (b & d) | (c & d);
    k = 0x8F1BBCDC;
  } else {
    f = b ^ c ^ d;
    k = 0xCA62C1D6;
  }

  uint temp = fd_sha1_rotl( a, 5 ) + f + e + k + w;
  uint new_c = fd_sha1_rotl( b, 30 );
  return temp ^ new_c;
}

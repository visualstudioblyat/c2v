/* fd_sha1_round — SHA-1 single round function.
   CHStone benchmark: sha (simplified to one round).
   Tests: bitwise rotations, if/else on round number, array lookup. */

typedef unsigned int uint;

uint fd_rotl32( uint x, uint n ) {
  return (x << n) | (x >> (32 - n));
}

uint
fd_sha1_round( uint a, uint b, uint c, uint d, uint e, uint w ) {
  /* Round 0-19: Ch function */
  uint f = (b & c) | ((~b) & d);
  uint k = 0x5A827999;

  uint temp = fd_rotl32( a, 5 ) + f + e + k + w;
  uint new_a = temp;
  uint new_c = fd_rotl32( b, 30 );

  return new_a ^ new_c;
}

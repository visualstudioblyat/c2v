/* fd_sha3_chi — SHA-3/Keccak Chi step on 5 lanes.
   Tests: bitwise NOT + AND + XOR.
   Real-world: blockchain, post-quantum crypto. */

typedef unsigned int uint;

uint
fd_sha3_chi( uint a0, uint a1, uint a2, uint a3, uint a4 ) {
  uint b0 = a0 ^ ((~a1) & a2);
  uint b1 = a1 ^ ((~a2) & a3);
  uint b2 = a2 ^ ((~a3) & a4);
  uint b3 = a3 ^ ((~a4) & a0);
  uint b4 = a4 ^ ((~a0) & a1);
  return b0 ^ b1 ^ b2 ^ b3 ^ b4;
}

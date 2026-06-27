typedef unsigned int uint;

uint
fd_fnv1a_4bytes( uint hash, uint b0, uint b1, uint b2, uint b3 ) {
  uint prime = 0x01000193;
  hash = (hash ^ b0) * prime;
  hash = (hash ^ b1) * prime;
  hash = (hash ^ b2) * prime;
  hash = (hash ^ b3) * prime;
  return hash;
}

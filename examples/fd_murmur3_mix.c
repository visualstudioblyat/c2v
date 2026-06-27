typedef unsigned int uint;

uint
fd_murmur3_fmix32( uint h ) {
  h = h ^ (h >> 16);
  h = h * 0x85EBCA6B;
  h = h ^ (h >> 13);
  h = h * 0xC2B2AE35;
  h = h ^ (h >> 16);
  return h;
}

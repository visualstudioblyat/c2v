/* fd_bloom_filter — Bloom filter hash + bit test.
   Tests: multiple hash functions + array bit manipulation.
   Real-world: network packet dedup, cache prefetching. */

typedef unsigned int uint;

uint fd_bloom_hash1( uint key ) {
  uint h = key;
  h = h ^ (h >> 16);
  h = h * 0x45d9f3b;
  h = h ^ (h >> 16);
  return h;
}

uint fd_bloom_hash2( uint key ) {
  uint h = key * 0x01000193;
  h = h ^ (h >> 13);
  h = h * 0x27d4eb2d;
  h = h ^ (h >> 15);
  return h;
}

uint
fd_bloom_test( uint key, uint filter_bits, uint filter_mask ) {
  uint h1 = fd_bloom_hash1( key ) & filter_mask;
  uint h2 = fd_bloom_hash2( key ) & filter_mask;
  uint h3 = (h1 + h2) & filter_mask;

  uint bit1 = (filter_bits >> (h1 & 31)) & 1;
  uint bit2 = (filter_bits >> (h2 & 31)) & 1;
  uint bit3 = (filter_bits >> (h3 & 31)) & 1;

  /* All bits must be set for "maybe present" */
  uint present = bit1 & bit2 & bit3;
  return (present << 31) | (h1 & 0x7FFFFFFF);
}

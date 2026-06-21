/* fd_scramble — multi-round bit permutation.
   Tests: deep inlining + loop + array + struct interaction. */

typedef unsigned int uint;

typedef struct {
  uint state;
  uint key;
} cipher_t;

uint fd_mix( uint x, uint k ) {
  x ^= k;
  x ^= (x >> 7);
  x *= 0x2c1b3c6d;
  x ^= (x >> 12);
  return x;
}

uint
fd_scramble( uint plaintext, uint key0, uint key1, uint key2, uint key3 ) {
  uint keys[4] = { key0, key1, key2, key3 };
  cipher_t ctx;
  ctx.state = plaintext;
  ctx.key = 0;

  for( int round = 0; round < 4; round++ ) {
    ctx.key = keys[round];
    ctx.state = fd_mix( ctx.state, ctx.key );
  }
  return ctx.state;
}

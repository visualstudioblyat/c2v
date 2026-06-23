/* fd_tile_ctx — Firedancer-style tile with context pointer.
   Tests: pointer-to-struct with multiple fields + computation.
   This is the canonical Firedancer tile function signature. */

typedef unsigned int  uint;
typedef unsigned long ulong;

typedef struct {
  uint  seq;
  uint  idx;
  uint  chunk0;
  uint  wmark;
  uint  out_chunk;
  uint  hashmap_seed;
} fd_dedup_ctx_t;

uint
fd_dedup_during_frag( fd_dedup_ctx_t *ctx,
                      uint sig,
                      uint chunk,
                      uint sz ) {
  /* Bounds check */
  uint valid = 1;
  if( chunk < ctx->chunk0 ) valid = 0;
  if( chunk > ctx->wmark )  valid = 0;
  if( sz > 1232 )           valid = 0;

  /* Hash for dedup */
  uint h = ctx->hashmap_seed;
  h = h ^ sig;
  h = h * 0x01000193;
  h = h ^ chunk;
  h = h * 0x01000193;

  /* Advance output chunk pointer */
  uint next_chunk = ctx->out_chunk + ((sz + 63) >> 6);
  if( next_chunk > ctx->wmark ) {
    next_chunk = ctx->chunk0;
  }

  return (valid << 31) | (h & 0x7FFFFFFF);
}

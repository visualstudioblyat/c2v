/* fd_dma_descriptor — DMA scatter-gather descriptor processor.
   Tests: struct pointer + if/else + arithmetic.
   Real-world: PCIe DMA engines, NIC packet processing. */

typedef unsigned int uint;

typedef struct {
  uint src_addr;
  uint dst_addr;
  uint length;
  uint flags;
  uint next_desc;
  uint status;
} dma_desc_t;

uint
fd_dma_process_desc( dma_desc_t *desc, uint current_addr ) {
  uint valid = 1;

  /* Validate descriptor */
  if( desc->length == 0 ) valid = 0;
  if( desc->length > 4096 ) valid = 0;
  if( desc->src_addr == 0 ) valid = 0;
  if( desc->dst_addr == 0 ) valid = 0;

  /* Compute transfer parameters */
  uint aligned_len = (desc->length + 63) & ~63;  /* 64-byte align */
  uint n_beats = aligned_len >> 6;                /* 64 bytes per beat */

  /* Update status */
  uint new_status;
  if( valid ) {
    new_status = 1;  /* ACTIVE */
  } else {
    new_status = 3;  /* ERROR */
  }

  /* Pack: status[3:0] | n_beats[11:0] | next_desc[15:0] */
  return (new_status << 28) | ((n_beats & 0xFFF) << 16) | (desc->next_desc & 0xFFFF);
}

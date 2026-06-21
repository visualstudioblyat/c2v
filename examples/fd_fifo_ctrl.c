/* fd_fifo_ctrl — FIFO pointer logic.
   Tests: struct + if/else + wrap-around arithmetic.
   Core building block for hardware FIFOs. */

typedef unsigned int uint;

typedef struct {
  uint rd_ptr;
  uint wr_ptr;
  uint depth;
  uint count;
} fifo_t;

uint
fd_fifo_push_pop( uint rd_ptr, uint wr_ptr,
                  uint depth, uint do_push, uint do_pop ) {
  fifo_t f;
  f.rd_ptr = rd_ptr;
  f.wr_ptr = wr_ptr;
  f.depth = depth;
  f.count = (f.wr_ptr - f.rd_ptr) & (f.depth - 1);

  uint new_wr = f.wr_ptr;
  uint new_rd = f.rd_ptr;

  if( do_push ) {
    new_wr = (f.wr_ptr + 1) & (f.depth - 1);
  }
  if( do_pop ) {
    new_rd = (f.rd_ptr + 1) & (f.depth - 1);
  }

  uint new_count = (new_wr - new_rd) & (f.depth - 1);
  uint full  = (new_count == (f.depth - 1)) ? 1 : 0;
  uint empty = (new_count == 0) ? 1 : 0;

  /* pack: new_wr[15:0] | new_rd[15:0] | full | empty */
  return (new_wr << 18) | (new_rd << 2) | (full << 1) | empty;
}

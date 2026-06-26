/* fd_vec_add — vector addition: c[i] = a[i] + b[i].
   The simplest possible pipelined loop. No carried deps.
   With loop pipelining: II=1, one result per clock.
   Without pipelining: N * body_latency cycles. */

typedef unsigned int uint;

uint
fd_vec_add_4( uint a0, uint a1, uint a2, uint a3,
              uint b0, uint b1, uint b2, uint b3 ) {
  uint c0 = a0 + b0;
  uint c1 = a1 + b1;
  uint c2 = a2 + b2;
  uint c3 = a3 + b3;
  return c0 + c1 + c2 + c3;
}

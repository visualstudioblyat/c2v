/* fd_matrix_det2x2 — 2x2 matrix determinant: ad - bc.
   AoC/competitive style: linear algebra kernel.
   Tests: 2 multiplies + 1 subtract → DSP48 inference.
   Competitive metric: should be 1 cycle pipelined. */

typedef unsigned int uint;

uint
fd_det2x2( uint a, uint b, uint c, uint d ) {
  return a * d - b * c;
}

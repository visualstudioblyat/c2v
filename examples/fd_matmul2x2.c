/* fd_matmul2x2 — 2x2 matrix multiply using arrays.
   A[2][2] * B[2][2] = C[2][2], flattened to 1D.
   4 multiply-accumulate operations, maps to DSPs + BRAM. */

typedef unsigned int uint;

uint
fd_matmul2x2( uint a00, uint a01, uint a10, uint a11,
              uint b00, uint b01, uint b10, uint b11 ) {
  uint a[4] = { a00, a01, a10, a11 };
  uint b[4] = { b00, b01, b10, b11 };

  /* c00 = a00*b00 + a01*b10 */
  uint c00 = a[0] * b[0] + a[1] * b[2];
  /* c01 = a00*b01 + a01*b11 */
  uint c01 = a[0] * b[1] + a[1] * b[3];
  /* c10 = a10*b00 + a11*b10 */
  uint c10 = a[2] * b[0] + a[3] * b[2];
  /* c11 = a10*b01 + a11*b11 */
  uint c11 = a[2] * b[1] + a[3] * b[3];

  /* Return trace (c00 + c11) as a checksum */
  return c00 + c01 + c10 + c11;
}

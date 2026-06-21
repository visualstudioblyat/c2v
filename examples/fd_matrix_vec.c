/* fd_matrix_vec — 4x4 matrix-vector multiply.
   Tests: nested loops + array indexing.
   Real-world: graphics, ML inference, DSP. */

typedef unsigned int uint;

uint
fd_mat4_vec4( uint m00, uint m01, uint m02, uint m03,
              uint m10, uint m11, uint m12, uint m13,
              uint m20, uint m21, uint m22, uint m23,
              uint m30, uint m31, uint m32, uint m33,
              uint v0, uint v1, uint v2, uint v3 ) {
  uint m[16] = { m00, m01, m02, m03,
                 m10, m11, m12, m13,
                 m20, m21, m22, m23,
                 m30, m31, m32, m33 };
  uint v[4] = { v0, v1, v2, v3 };
  uint result[4] = { 0, 0, 0, 0 };

  for( int i = 0; i < 4; i++ ) {
    for( int j = 0; j < 4; j++ ) {
      result[i] = result[i] + m[i * 4 + j] * v[j];
    }
  }

  /* pack first two results */
  return (result[0] << 16) | (result[1] & 0xFFFF);
}

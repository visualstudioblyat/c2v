/* fd_struct_ptr — pointer-to-struct parameter.
   Tests: struct field access via pointer (ctx->field).
   This is how every real Firedancer function is written. */

typedef unsigned int uint;

typedef struct {
  uint x;
  uint y;
  uint z;
} vec3_t;

uint
fd_vec3_length_sq( vec3_t *v ) {
  return v->x * v->x + v->y * v->y + v->z * v->z;
}

/* fd_nested_struct — struct containing another struct.
   Tests nested field access: outer.inner.field */

typedef unsigned int uint;

typedef struct {
  uint x;
  uint y;
} vec2_t;

typedef struct {
  vec2_t pos;
  vec2_t vel;
  uint mass;
} body_t;

uint
fd_body_energy( uint px, uint py,
                uint vx, uint vy,
                uint mass ) {
  body_t b;
  b.pos.x = px;
  b.pos.y = py;
  b.vel.x = vx;
  b.vel.y = vy;
  b.mass = mass;

  /* kinetic energy: 0.5 * m * v^2  (integer approx) */
  uint v2 = b.vel.x * b.vel.x + b.vel.y * b.vel.y;
  uint ke = (b.mass * v2) >> 1;

  /* potential energy: m * g * h  (g=10, h=pos.y) */
  uint pe = b.mass * 10 * b.pos.y;

  return ke + pe;
}

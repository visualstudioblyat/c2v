/* fd_point — struct field access and computation.
   POD struct, field reads/writes, arithmetic on fields. */

typedef unsigned int uint;

typedef struct {
  uint x;
  uint y;
  uint z;
} point_t;

uint
fd_point_dot( uint ax, uint ay, uint az,
              uint bx, uint by, uint bz ) {
  point_t a;
  a.x = ax;
  a.y = ay;
  a.z = az;

  point_t b;
  b.x = bx;
  b.y = by;
  b.z = bz;

  return a.x * b.x + a.y * b.y + a.z * b.z;
}

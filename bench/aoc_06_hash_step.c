/* AoC-style: hash-based position update (grid walk).
   Simulates "guard patrol" direction logic.
   Tests: conditional state machine, modular arithmetic.
   Metric: cycles per step. */

typedef unsigned int uint;

uint
aoc_guard_step( uint x, uint y, uint dir, uint blocked ) {
  uint nx = x;
  uint ny = y;
  uint ndir = dir;

  /* dir: 0=up 1=right 2=down 3=left */
  if( !blocked ) {
    if( dir == 0 ) ny = y - 1;
    if( dir == 1 ) nx = x + 1;
    if( dir == 2 ) ny = y + 1;
    if( dir == 3 ) nx = x - 1;
  } else {
    ndir = (dir + 1) & 3;
  }

  return (ndir << 24) | ((ny & 0xFFF) << 12) | (nx & 0xFFF);
}

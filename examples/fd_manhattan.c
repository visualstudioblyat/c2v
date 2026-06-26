/* fd_manhattan — Manhattan distance: |x1-x2| + |y1-y2|.
   AoC classic: grid distance used in pathfinding.
   Tests: subtraction + absolute value + addition.
   Competitive metric: 1-2 cycles. */

typedef unsigned int uint;

uint
fd_manhattan( uint x1, uint y1, uint x2, uint y2 ) {
  uint dx = (x1 > x2) ? (x1 - x2) : (x2 - x1);
  uint dy = (y1 > y2) ? (y1 - y2) : (y2 - y1);
  return dx + dy;
}

/* fd_cross_product — 3D cross product: c = a × b.
   AoC/competitive: geometry kernel used in 3D graphics.
   Tests: 6 multiplies + 3 subtracts → DSP inference.
   Competitive metric: 2 DSP slices (packed), 1 cycle pipelined. */

typedef unsigned int uint;

uint
fd_cross_z( uint ax, uint ay, uint bx, uint by ) {
  return ax * by - ay * bx;
}

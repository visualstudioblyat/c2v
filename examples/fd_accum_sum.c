typedef unsigned int uint;

uint
fd_accum_add( uint val, uint prev_sum ) {
  return prev_sum + val;
}

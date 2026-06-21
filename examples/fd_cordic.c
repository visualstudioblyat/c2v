/* fd_cordic — CORDIC rotation, 8 iterations.
   Classic FPGA algorithm: computes sin/cos approximation
   using only shifts and adds. No multipliers needed. */

typedef unsigned int uint;
typedef int sint;

uint
fd_cordic( sint x_in, sint y_in, sint angle ) {
  sint x = x_in;
  sint y = y_in;
  sint z = angle;

  /* atan table (fixed-point, scaled by 2^14) */
  uint atan_table[8] = { 8192, 4836, 2555, 1297, 651, 326, 163, 81 };

  for( int i = 0; i < 8; i++ ) {
    sint x_new;
    sint y_new;
    if( z >= 0 ) {
      x_new = x - (y >> i);
      y_new = y + (x >> i);
      z = z - atan_table[i];
    } else {
      x_new = x + (y >> i);
      y_new = y - (x >> i);
      z = z + atan_table[i];
    }
    x = x_new;
    y = y_new;
  }

  /* pack x and y into result */
  return ((x & 0xFFFF) << 16) | (y & 0xFFFF);
}

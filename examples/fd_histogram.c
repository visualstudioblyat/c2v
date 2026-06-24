/* fd_histogram — histogram bin counting.
   Tests: array read-modify-write + loop.
   Real-world: image analysis, statistics, ML preprocessing. */

typedef unsigned int uint;

uint
fd_histogram_4bin( uint v0, uint v1, uint v2, uint v3,
                   uint v4, uint v5, uint v6, uint v7 ) {
  uint bins[4] = { 0, 0, 0, 0 };
  uint values[8] = { v0, v1, v2, v3, v4, v5, v6, v7 };

  for( int i = 0; i < 8; i++ ) {
    uint bin = (values[i] >> 6) & 3;
    if( bin == 0 )      bins[0] = bins[0] + 1;
    else if( bin == 1 ) bins[1] = bins[1] + 1;
    else if( bin == 2 ) bins[2] = bins[2] + 1;
    else                bins[3] = bins[3] + 1;
  }

  return (bins[3] << 24) | (bins[2] << 16) | (bins[1] << 8) | bins[0];
}

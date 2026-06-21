/* fd_lut — lookup table with computation.
   Array read + arithmetic. Maps to BRAM + datapath. */

typedef unsigned int uint;

uint
fd_lut_compute( uint idx,
                uint scale ) {
  uint table[8] = { 1, 3, 7, 15, 31, 63, 127, 255 };
  uint val = table[idx & 7];
  uint result = val * scale;
  return result;
}

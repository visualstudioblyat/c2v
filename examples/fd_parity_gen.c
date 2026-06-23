/* fd_parity_gen — Hamming(7,4) parity generator.
   Tests: bitwise XOR + bit extraction.
   Real-world: ECC for memory, NAND flash, communication links. */

typedef unsigned int uint;

uint
fd_hamming_encode( uint data ) {
  uint d = data & 0xF;
  uint d0 = d & 1;
  uint d1 = (d >> 1) & 1;
  uint d2 = (d >> 2) & 1;
  uint d3 = (d >> 3) & 1;

  uint p0 = d0 ^ d1 ^ d3;
  uint p1 = d0 ^ d2 ^ d3;
  uint p2 = d1 ^ d2 ^ d3;

  return (p0) | (p1 << 1) | (d0 << 2) | (p2 << 3) |
         (d1 << 4) | (d2 << 5) | (d3 << 6);
}

uint
fd_hamming_check( uint codeword ) {
  uint c = codeword & 0x7F;
  uint s0 = ((c >> 0) ^ (c >> 2) ^ (c >> 4) ^ (c >> 6)) & 1;
  uint s1 = ((c >> 1) ^ (c >> 2) ^ (c >> 5) ^ (c >> 6)) & 1;
  uint s2 = ((c >> 3) ^ (c >> 4) ^ (c >> 5) ^ (c >> 6)) & 1;
  uint syndrome = (s2 << 2) | (s1 << 1) | s0;

  /* Correct single-bit error */
  uint corrected = c;
  if( syndrome != 0 ) {
    corrected = c ^ (1 << (syndrome - 1));
  }

  /* Extract data bits */
  uint d0 = (corrected >> 2) & 1;
  uint d1 = (corrected >> 4) & 1;
  uint d2 = (corrected >> 5) & 1;
  uint d3 = (corrected >> 6) & 1;

  return (syndrome << 4) | (d3 << 3) | (d2 << 2) | (d1 << 1) | d0;
}

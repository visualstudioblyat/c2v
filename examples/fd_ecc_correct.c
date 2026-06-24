/* fd_ecc_correct — Hamming(7,4) single-error correction.
   Tests: parity computation + syndrome decode + bit flip.
   Real-world: memory ECC, flash storage, communication links. */

typedef unsigned int uint;

uint
fd_hamming74_correct( uint codeword ) {
  /* Codeword bits: d3 d2 d1 d0 p2 p1 p0 */
  uint c = codeword & 0x7F;

  uint d3 = (c >> 6) & 1;
  uint d2 = (c >> 5) & 1;
  uint d1 = (c >> 4) & 1;
  uint d0 = (c >> 3) & 1;
  uint p2 = (c >> 2) & 1;
  uint p1 = (c >> 1) & 1;
  uint p0 =  c       & 1;

  /* Compute syndrome */
  uint s0 = p0 ^ d0 ^ d1 ^ d3;
  uint s1 = p1 ^ d0 ^ d2 ^ d3;
  uint s2 = p2 ^ d1 ^ d2 ^ d3;
  uint syndrome = (s2 << 2) | (s1 << 1) | s0;

  /* Correct single-bit error */
  uint corrected = c;
  if( syndrome != 0 ) {
    corrected = c ^ (1 << (syndrome - 1));
  }

  /* Extract corrected data bits */
  uint data = ((corrected >> 3) & 0xF);

  return (syndrome << 4) | data;
}

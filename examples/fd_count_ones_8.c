/* fd_count_ones_8 — population count of 8-element array.
   AoC-style: reduction over array → tree balancing opportunity.
   Tests: array load + addition chain.
   With tree balancing: depth 3 instead of 7. */

typedef unsigned int uint;

uint
fd_count_ones_8( uint a0, uint a1, uint a2, uint a3,
                 uint a4, uint a5, uint a6, uint a7 ) {
  uint c0 = a0 & 1;
  uint c1 = a1 & 1;
  uint c2 = a2 & 1;
  uint c3 = a3 & 1;
  uint c4 = a4 & 1;
  uint c5 = a5 & 1;
  uint c6 = a6 & 1;
  uint c7 = a7 & 1;

  return c0 + c1 + c2 + c3 + c4 + c5 + c6 + c7;
}

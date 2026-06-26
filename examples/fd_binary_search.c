/* fd_binary_search — binary search over sorted 16-element array.
   AoC classic: search algorithm → hardware comparison tree.
   Tests: array load + cascaded if/else + data-dependent path.
   Competitive metric: log2(16)=4 cycle depth. */

typedef unsigned int uint;

uint
fd_binary_search_16( uint key,
                     uint a0,  uint a1,  uint a2,  uint a3,
                     uint a4,  uint a5,  uint a6,  uint a7,
                     uint a8,  uint a9,  uint a10, uint a11,
                     uint a12, uint a13, uint a14, uint a15 ) {
  uint arr[16] = { a0, a1, a2, a3, a4, a5, a6, a7,
                   a8, a9, a10, a11, a12, a13, a14, a15 };

  uint lo = 0;
  uint hi = 15;
  uint found = 0;
  uint result = 16;

  for( int iter = 0; iter < 4; iter++ ) {
    uint mid = (lo + hi) >> 1;
    uint val = arr[mid];
    if( val == key ) {
      result = mid;
      found = 1;
    } else if( val < key ) {
      lo = mid + 1;
    } else {
      hi = mid;
    }
  }

  return (found << 31) | result;
}

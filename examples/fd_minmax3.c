/* fd_minmax3 — find min and max of 3 values using if/else.
   Returns packed: (min << 16) | max.
   Tests multiple variables modified in branches. */

typedef unsigned int uint;

uint
fd_minmax3( uint a, uint b, uint c ) {
  uint mn = a;
  uint mx = a;

  if( b < mn ) mn = b;
  if( b > mx ) mx = b;
  if( c < mn ) mn = c;
  if( c > mx ) mx = c;

  return (mn << 16) | mx;
}

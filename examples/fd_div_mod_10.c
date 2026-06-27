typedef unsigned int uint;

uint
fd_div10( uint n ) {
  uint q = (n >> 1) + (n >> 2);
  q = q + (q >> 4);
  q = q + (q >> 8);
  q = q + (q >> 16);
  q = q >> 3;
  uint r = n - q * 10;
  return (r >= 10) ? q + 1 : q;
}

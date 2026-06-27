typedef unsigned int uint;

uint
fd_xxh32_round( uint acc, uint input ) {
  uint prime1 = 0x9E3779B1;
  uint prime2 = 0x85EBCA77;
  acc = acc + input * prime2;
  acc = (acc << 13) | (acc >> 19);
  acc = acc * prime1;
  return acc;
}

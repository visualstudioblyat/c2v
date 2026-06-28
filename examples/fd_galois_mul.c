typedef unsigned int uint;

uint
fd_gf256_mul( uint a, uint b ) {
  a = a & 0xFF;
  b = b & 0xFF;
  uint p = 0;
  uint carry;
  carry = (a >> 7) & 1; a = ((a << 1) & 0xFF) ^ (carry ? 0x1B : 0); if (b & 1) p = p ^ a;
  carry = (a >> 7) & 1; a = ((a << 1) & 0xFF) ^ (carry ? 0x1B : 0); if (b & 2) p = p ^ a;
  carry = (a >> 7) & 1; a = ((a << 1) & 0xFF) ^ (carry ? 0x1B : 0); if (b & 4) p = p ^ a;
  carry = (a >> 7) & 1; a = ((a << 1) & 0xFF) ^ (carry ? 0x1B : 0); if (b & 8) p = p ^ a;
  carry = (a >> 7) & 1; a = ((a << 1) & 0xFF) ^ (carry ? 0x1B : 0); if (b & 16) p = p ^ a;
  carry = (a >> 7) & 1; a = ((a << 1) & 0xFF) ^ (carry ? 0x1B : 0); if (b & 32) p = p ^ a;
  carry = (a >> 7) & 1; a = ((a << 1) & 0xFF) ^ (carry ? 0x1B : 0); if (b & 64) p = p ^ a;
  carry = (a >> 7) & 1; a = ((a << 1) & 0xFF) ^ (carry ? 0x1B : 0); if (b & 128) p = p ^ a;
  return p & 0xFF;
}

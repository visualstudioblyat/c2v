/* fd_hash_round — SHA-256-like round function, Firedancer style */

typedef unsigned int uint;

uint
fd_hash_round( uint a,
               uint b,
               uint c,
               uint d,
               uint e,
               uint f,
               uint g,
               uint h,
               uint k,
               uint w ) {
  uint s1  = ((e >> 6) ^ (e >> 11) ^ (e >> 25));
  uint ch  = ((e & f) ^ ((~e) & g));
  uint t1  = h + s1 + ch + k + w;
  uint s0  = ((a >> 2) ^ (a >> 13) ^ (a >> 22));
  uint maj = ((a & b) ^ (a & c) ^ (b & c));
  uint t2  = s0 + maj;

  return d + t1 + t2;
}

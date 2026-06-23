/* fd_ntt_butterfly — Number Theoretic Transform butterfly.
   Tests: modular arithmetic + multiply.
   Real-world: post-quantum crypto, polynomial multiplication, ZK proofs. */

typedef unsigned int uint;

uint
fd_ntt_butterfly( uint a, uint b, uint w, uint p ) {
  /* Montgomery-like: t = b * w mod p (simplified, no actual mod) */
  uint t_lo = b * w;
  uint t = t_lo; /* In real NTT this would be Barrett/Montgomery reduction */

  /* Butterfly: out_a = a + t, out_b = a - t (mod p) */
  uint sum = a + t;
  uint diff;
  if( a >= t ) {
    diff = a - t;
  } else {
    diff = a + p - t; /* wrap around */
  }

  /* Reduce if needed */
  if( sum >= p ) sum = sum - p;

  return (sum << 16) | (diff & 0xFFFF);
}

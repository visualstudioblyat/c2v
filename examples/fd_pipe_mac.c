/* fd_pipe_mac — pipelined multiply-accumulate.
   Each call does: result = a*b + c*d + prev
   This tests that the compiler can handle accumulation
   across function invocations (pipeline stages). */

typedef unsigned int uint;

uint
fd_pipe_mac( uint a, uint b,
             uint c, uint d,
             uint prev ) {
  uint p0 = a * b;
  uint p1 = c * d;
  uint sum = p0 + p1;
  return sum + prev;
}

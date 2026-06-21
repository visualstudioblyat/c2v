/* fd_alu — simple ALU with opcode dispatch.
   Tests: chained if/else as opcode decoder → mux tree. */

typedef unsigned int uint;

uint
fd_alu( uint a, uint b, uint op ) {
  uint result;
  if( op == 0 ) {
    result = a + b;
  } else if( op == 1 ) {
    result = a - b;
  } else if( op == 2 ) {
    result = a & b;
  } else if( op == 3 ) {
    result = a | b;
  } else if( op == 4 ) {
    result = a ^ b;
  } else if( op == 5 ) {
    result = a << (b & 31);
  } else if( op == 6 ) {
    result = a >> (b & 31);
  } else {
    result = a;
  }
  return result;
}

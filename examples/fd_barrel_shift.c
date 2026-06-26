/* fd_barrel_shift — barrel shifter with direction select.
   Tests: conditional shift direction + mask.
   Real-world: ALU, floating point normalizer, crypto rotation. */

typedef unsigned int uint;

uint
fd_barrel_shift( uint data, uint amount, uint direction ) {
  uint amt = amount & 31;
  uint result;

  if( direction == 0 ) {
    result = data << amt;
  } else if( direction == 1 ) {
    result = data >> amt;
  } else {
    /* Rotate left */
    result = (data << amt) | (data >> (32 - amt));
  }

  return result;
}

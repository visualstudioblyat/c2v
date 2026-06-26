/* fd_round_robin — 4-port round-robin arbiter with priority mask.
   Tests: multi-way if/else + counter + bit rotation.
   Real-world: crossbar switch, shared bus arbiter, scheduler. */

typedef unsigned int uint;

uint
fd_rr_arbiter( uint requests, uint last_grant, uint mask ) {
  uint req = requests & mask;
  uint start = (last_grant + 1) & 3;
  uint grant = 0;
  uint found = 0;

  /* Search from start position, wrap around */
  for( int i = 0; i < 4; i++ ) {
    uint pos = (start + i) & 3;
    if( !found && ((req >> pos) & 1) ) {
      grant = pos;
      found = 1;
    }
  }

  return (found << 31) | (grant & 3);
}

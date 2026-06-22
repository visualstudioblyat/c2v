/* fd_arbiter — round-robin arbiter for 4 requestors.
   Tests: if/else chain + state tracking + priority logic.
   Real-world: bus arbitration, NoC routing. */

typedef unsigned int uint;

uint
fd_arbiter_rr4( uint req, uint last_grant ) {
  uint grant = 0;
  uint r = req & 0xF;
  uint next_last = last_grant & 3;

  /* Round-robin: start from the requester after last grant */
  uint start = (next_last + 1) & 3;

  if( (r >> start) & 1 ) {
    grant = 1 << start;
    next_last = start;
  } else if( (r >> ((start + 1) & 3)) & 1 ) {
    uint idx = (start + 1) & 3;
    grant = 1 << idx;
    next_last = idx;
  } else if( (r >> ((start + 2) & 3)) & 1 ) {
    uint idx = (start + 2) & 3;
    grant = 1 << idx;
    next_last = idx;
  } else if( (r >> ((start + 3) & 3)) & 1 ) {
    uint idx = (start + 3) & 3;
    grant = 1 << idx;
    next_last = idx;
  }

  return (grant << 4) | next_last;
}

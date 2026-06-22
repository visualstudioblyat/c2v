/* fd_arbiter — round-robin arbiter for 4 requestors.
   Tests: chained if/else with computed shifts.
   Real-world: bus arbitration, NoC routing. */

typedef unsigned int uint;

uint
fd_arbiter_rr4( uint req, uint last_grant ) {
  uint r = req & 0xF;
  uint lg = last_grant & 3;
  uint start = (lg + 1) & 3;

  uint s0 = start;
  uint s1 = (start + 1) & 3;
  uint s2 = (start + 2) & 3;
  uint s3 = (start + 3) & 3;

  uint grant = 0;
  uint next_last = lg;

  if( (r >> s0) & 1 ) {
    grant = 1 << s0;
    next_last = s0;
  } else if( (r >> s1) & 1 ) {
    grant = 1 << s1;
    next_last = s1;
  } else if( (r >> s2) & 1 ) {
    grant = 1 << s2;
    next_last = s2;
  } else if( (r >> s3) & 1 ) {
    grant = 1 << s3;
    next_last = s3;
  }

  return (grant << 4) | next_last;
}

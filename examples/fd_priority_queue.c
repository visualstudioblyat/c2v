typedef unsigned int uint;

uint
fd_pq_insert_sorted4( uint q0, uint q1, uint q2, uint q3, uint val ) {
  uint p3 = (val > q3) ? q3 : val;
  uint v3 = (val > q3) ? val : q3;

  uint p2 = (v3 > q2) ? q2 : v3;
  uint v2 = (v3 > q2) ? v3 : q2;

  uint p1 = (v2 > q1) ? q1 : v2;
  uint v1 = (v2 > q1) ? v2 : q1;

  uint p0 = (v1 > q0) ? q0 : v1;

  return (p0 << 24) | ((p1 & 0xFF) << 16) | ((p2 & 0xFF) << 8) | (p3 & 0xFF);
}

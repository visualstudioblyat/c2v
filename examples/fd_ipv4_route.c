typedef unsigned int uint;

uint
fd_lpm_match( uint ip, uint prefix, uint mask, uint next_hop ) {
  uint masked_ip = ip & mask;
  uint masked_pfx = prefix & mask;
  uint match = (masked_ip == masked_pfx) ? 1 : 0;
  return match ? next_hop : 0;
}

uint
fd_route_lookup_4( uint ip,
                   uint pfx0, uint mask0, uint hop0,
                   uint pfx1, uint mask1, uint hop1,
                   uint pfx2, uint mask2, uint hop2,
                   uint pfx3, uint mask3, uint hop3 ) {
  uint r0 = fd_lpm_match(ip, pfx0, mask0, hop0);
  uint r1 = fd_lpm_match(ip, pfx1, mask1, hop1);
  uint r2 = fd_lpm_match(ip, pfx2, mask2, hop2);
  uint r3 = fd_lpm_match(ip, pfx3, mask3, hop3);
  uint best = r0;
  if( r1 ) best = r1;
  if( r2 ) best = r2;
  if( r3 ) best = r3;
  return best;
}

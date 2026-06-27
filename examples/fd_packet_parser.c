typedef unsigned int uint;

uint
fd_parse_eth_type( uint hdr0, uint hdr1, uint hdr2, uint hdr3 ) {
  uint dst_hi  = hdr0;
  uint dst_lo  = hdr1 >> 16;
  uint src_hi  = (hdr1 & 0xFFFF) << 16 | (hdr2 >> 16);
  uint src_lo  = hdr2 & 0xFFFF;
  uint ethertype = hdr3 >> 16;

  uint is_ipv4 = (ethertype == 0x0800) ? 1 : 0;
  uint is_ipv6 = (ethertype == 0x86DD) ? 1 : 0;
  uint is_arp  = (ethertype == 0x0806) ? 1 : 0;
  uint is_vlan = (ethertype == 0x8100) ? 1 : 0;

  return (is_vlan << 3) | (is_arp << 2) | (is_ipv6 << 1) | is_ipv4;
}

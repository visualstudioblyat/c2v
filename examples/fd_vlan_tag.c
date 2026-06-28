typedef unsigned int uint;

uint
fd_vlan_push( uint ethertype, uint vlan_id, uint pcp, uint dei ) {
  uint tpid = 0x8100;
  uint tci = ((pcp & 0x7) << 13) | ((dei & 0x1) << 12) | (vlan_id & 0xFFF);
  return (tpid << 16) | tci;
}

uint
fd_vlan_pop( uint vlan_header ) {
  uint vlan_id = vlan_header & 0xFFF;
  uint pcp = (vlan_header >> 13) & 0x7;
  uint dei = (vlan_header >> 12) & 0x1;
  return (pcp << 16) | (dei << 15) | vlan_id;
}

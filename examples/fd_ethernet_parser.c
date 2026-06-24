/* fd_ethernet_parser — Ethernet frame header parser.
   Tests: struct pointer + bitwise extraction + enum.
   Real-world: network processing on SmartNIC FPGA. */

typedef unsigned int uint;

enum ethertype {
  ETH_IPV4  = 0x0800,
  ETH_IPV6  = 0x86DD,
  ETH_ARP   = 0x0806,
  ETH_VLAN  = 0x8100
};

typedef struct {
  uint dst_mac_hi;
  uint dst_mac_lo;
  uint src_mac_hi;
  uint src_mac_lo;
  uint ethertype;
  uint payload_word0;
} eth_header_t;

uint
fd_eth_classify( eth_header_t *hdr ) {
  uint etype = hdr->ethertype & 0xFFFF;
  uint is_broadcast = (hdr->dst_mac_hi == 0xFFFFFFFF) ? 1 : 0;

  uint pkt_class;
  if( etype == ETH_IPV4 ) {
    pkt_class = 1;
  } else if( etype == ETH_IPV6 ) {
    pkt_class = 2;
  } else if( etype == ETH_ARP ) {
    pkt_class = 3;
  } else if( etype == ETH_VLAN ) {
    pkt_class = 4;
  } else {
    pkt_class = 0;
  }

  /* Extract IP protocol if IPv4 */
  uint ip_proto = 0;
  if( pkt_class == 1 ) {
    ip_proto = (hdr->payload_word0 >> 16) & 0xFF;
  }

  return (is_broadcast << 24) | (pkt_class << 16) | (ip_proto << 8) | (etype & 0xFF);
}

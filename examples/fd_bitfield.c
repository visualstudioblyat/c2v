typedef unsigned int uint;

uint
fd_pack_ipv4_hdr( uint version, uint ihl, uint dscp, uint ecn,
                  uint total_len, uint identification,
                  uint flags, uint frag_offset, uint ttl, uint protocol ) {
  uint w0 = ((version & 0xF) << 28) |
            ((ihl & 0xF) << 24) |
            ((dscp & 0x3F) << 18) |
            ((ecn & 0x3) << 16) |
            (total_len & 0xFFFF);
  uint w1 = ((identification & 0xFFFF) << 16) |
            ((flags & 0x7) << 13) |
            (frag_offset & 0x1FFF);
  uint w2 = ((ttl & 0xFF) << 24) |
            ((protocol & 0xFF) << 16);
  return w0 ^ w1 ^ w2;
}

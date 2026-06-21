/* fd_packet — packet header parsing and checksum.
   Struct with multiple fields, bitwise extraction, accumulation.
   Typical network FPGA workload. */

typedef unsigned int  uint;

typedef struct {
  uint src_ip;
  uint dst_ip;
  uint src_port;
  uint dst_port;
  uint protocol;
  uint length;
} pkt_hdr_t;

uint
fd_packet_checksum( uint src_ip, uint dst_ip,
                    uint src_port, uint dst_port,
                    uint protocol, uint length ) {
  pkt_hdr_t hdr;
  hdr.src_ip   = src_ip;
  hdr.dst_ip   = dst_ip;
  hdr.src_port = src_port;
  hdr.dst_port = dst_port;
  hdr.protocol = protocol;
  hdr.length   = length;

  /* ones-complement sum of all fields */
  uint sum = hdr.src_ip;
  sum = sum + hdr.dst_ip;
  sum = sum + ((hdr.src_port << 16) | hdr.dst_port);
  sum = sum + ((hdr.protocol << 16) | hdr.length);

  /* fold carry */
  sum = (sum >> 16) + (sum & 0xFFFF);
  sum = (sum >> 16) + (sum & 0xFFFF);
  return ~sum & 0xFFFF;
}

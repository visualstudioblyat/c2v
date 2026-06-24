/* fd_tcp_checksum — TCP/IP ones-complement checksum (4 words).
   Tests: loop + carry propagation + ones-complement fold.
   Real-world: NIC offload, firewall, load balancer FPGA. */

typedef unsigned int uint;

uint
fd_tcp_checksum_4w( uint w0, uint w1, uint w2, uint w3 ) {
  uint words[4] = { w0, w1, w2, w3 };
  uint sum = 0;

  for( int i = 0; i < 4; i++ ) {
    sum = sum + (words[i] & 0xFFFF);
    sum = sum + (words[i] >> 16);
  }

  /* Fold 32-bit sum to 16 bits */
  sum = (sum >> 16) + (sum & 0xFFFF);
  sum = (sum >> 16) + (sum & 0xFFFF);

  return (~sum) & 0xFFFF;
}

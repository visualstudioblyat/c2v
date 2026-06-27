typedef unsigned int uint;

uint
fd_ip_checksum( uint w0, uint w1, uint w2, uint w3, uint w4,
                uint w5, uint w6, uint w7, uint w8, uint w9 ) {
  uint sum = 0;
  sum += w0 & 0xFFFF; sum += w0 >> 16;
  sum += w1 & 0xFFFF; sum += w1 >> 16;
  sum += w2 & 0xFFFF; sum += w2 >> 16;
  sum += w3 & 0xFFFF; sum += w3 >> 16;
  sum += w4 & 0xFFFF; sum += w4 >> 16;
  sum += w5 & 0xFFFF; sum += w5 >> 16;
  sum += w6 & 0xFFFF; sum += w6 >> 16;
  sum += w7 & 0xFFFF; sum += w7 >> 16;
  sum += w8 & 0xFFFF; sum += w8 >> 16;
  sum += w9 & 0xFFFF; sum += w9 >> 16;
  sum = (sum >> 16) + (sum & 0xFFFF);
  sum = (sum >> 16) + (sum & 0xFFFF);
  return (~sum) & 0xFFFF;
}

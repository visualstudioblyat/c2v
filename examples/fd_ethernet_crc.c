/* fd_ethernet_crc — Ethernet CRC-32 (1 byte at a time).
   Tests: loop + conditional XOR + table lookup.
   Real-world: network FPGA, NIC offload. */

typedef unsigned int uint;

uint
fd_eth_crc32_byte( uint crc, uint byte ) {
  uint table[16] = {
    0x00000000, 0x1DB71064, 0x3B6E20C8, 0x26D930AC,
    0x76DC4190, 0x6B6B51F4, 0x4DB26158, 0x5005713C,
    0xEDB88320, 0xF00F9344, 0xD6D6A3E8, 0xCB61B38C,
    0x9B64C2B0, 0x86D3D2D4, 0xA00AE278, 0xBDBDF21C
  };

  uint c = crc ^ byte;
  /* Process low nibble */
  uint idx0 = c & 0xF;
  c = (c >> 4) ^ table[idx0];
  /* Process high nibble */
  uint idx1 = c & 0xF;
  c = (c >> 4) ^ table[idx1];
  return c;
}

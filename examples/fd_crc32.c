typedef unsigned int uint;

uint
fd_crc32_byte( uint crc, uint byte ) {
  crc = crc ^ byte;
  uint b0 = (crc & 1) ? 0xEDB88320 : 0; crc = (crc >> 1) ^ b0;
  uint b1 = (crc & 1) ? 0xEDB88320 : 0; crc = (crc >> 1) ^ b1;
  uint b2 = (crc & 1) ? 0xEDB88320 : 0; crc = (crc >> 1) ^ b2;
  uint b3 = (crc & 1) ? 0xEDB88320 : 0; crc = (crc >> 1) ^ b3;
  uint b4 = (crc & 1) ? 0xEDB88320 : 0; crc = (crc >> 1) ^ b4;
  uint b5 = (crc & 1) ? 0xEDB88320 : 0; crc = (crc >> 1) ^ b5;
  uint b6 = (crc & 1) ? 0xEDB88320 : 0; crc = (crc >> 1) ^ b6;
  uint b7 = (crc & 1) ? 0xEDB88320 : 0; crc = (crc >> 1) ^ b7;
  return crc;
}

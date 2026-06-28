typedef unsigned int uint;

uint
fd_utf8_char_len( uint byte ) {
  byte = byte & 0xFF;
  if( (byte & 0x80) == 0 )    return 1;
  if( (byte & 0xE0) == 0xC0 ) return 2;
  if( (byte & 0xF0) == 0xE0 ) return 3;
  if( (byte & 0xF8) == 0xF0 ) return 4;
  return 0;
}

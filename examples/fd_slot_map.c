/* fd_slot_map — slot map allocator (find first free slot).
   Tests: loop + if/break pattern (unrolled) + bit scanning.
   Real-world: connection tracking, buffer management, TLB. */

typedef unsigned int uint;

uint
fd_find_free_slot( uint bitmap ) {
  uint slot = 0;
  uint found = 0;

  for( int i = 0; i < 32; i++ ) {
    if( !found && !((bitmap >> i) & 1) ) {
      slot = i;
      found = 1;
    }
  }

  return (found << 31) | (slot & 0x1F);
}

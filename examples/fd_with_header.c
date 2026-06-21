/* fd_with_header — uses #include and macros.
   Compile with: c2v --preprocess examples/fd_with_header.c -f fd_compute */

#include "fd_macro.h"

uint
fd_compute( uint a, uint b, uint c ) {
  uint r = fd_uint_rotate_left( a, 5 );
  uint aligned = FD_ULONG_ALIGN_UP( b, 16 );
  return r ^ aligned ^ c;
}

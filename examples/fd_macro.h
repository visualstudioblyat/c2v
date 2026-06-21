#ifndef FD_MACRO_H
#define FD_MACRO_H

#define FD_LIKELY(c)   __builtin_expect( !!(c), 1L )
#define FD_UNLIKELY(c) __builtin_expect( !!(c), 0L )

typedef unsigned int  uint;
typedef unsigned long ulong;

#define FD_ULONG_ALIGN_UP(x,a) (((x)+(a)-1) & ~((a)-1))

static inline uint
fd_uint_rotate_left( uint x, int n ) {
  return (x << n) | (x >> (32 - n));
}

#endif

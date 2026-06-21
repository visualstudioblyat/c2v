/* fd_bad_malloc — this should be REJECTED by the coding standard */

#include <stdlib.h>

int bad_function( int n ) {
  int *p = malloc( n * sizeof(int) );
  for( int i = 0; i < n; i++ ) {
    p[i] = i * i;
  }
  int sum = 0;
  for( int i = 0; i < n; i++ ) {
    sum += p[i];
  }
  free( p );
  return sum;
}

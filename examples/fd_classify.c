/* fd_classify — nested if/else chain acting as classifier.
   Maps priority values to weight categories. */

typedef unsigned int uint;

uint
fd_classify( uint priority, uint value ) {
  uint weight;
  if( priority == 0 ) {
    weight = value * 4;
  } else if( priority == 1 ) {
    weight = value * 2;
  } else if( priority == 2 ) {
    weight = value;
  } else {
    weight = value >> 1;
  }
  return weight + priority;
}

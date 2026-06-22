/* fd_pack_schedule — transaction scheduling/packing logic.
   Tests: priority comparison + struct + if/else chain.
   Real-world: Firedancer pack tile. */

typedef unsigned int uint;

typedef struct {
  uint fee;
  uint compute_units;
  uint priority;
  uint age;
} txn_meta_t;

uint
fd_pack_compare( uint fee_a, uint cu_a, uint age_a,
                 uint fee_b, uint cu_b, uint age_b,
                 uint cu_limit ) {
  txn_meta_t a;
  a.fee = fee_a;
  a.compute_units = cu_a;
  a.age = age_a;
  /* priority = fee per compute unit (simplified) */
  a.priority = fee_a;

  txn_meta_t b;
  b.fee = fee_b;
  b.compute_units = cu_b;
  b.age = age_b;
  b.priority = fee_b;

  /* Check if either exceeds CU limit */
  uint a_fits = (a.compute_units <= cu_limit) ? 1 : 0;
  uint b_fits = (b.compute_units <= cu_limit) ? 1 : 0;

  uint winner; /* 0=a wins, 1=b wins, 2=neither fits */

  if( a_fits && b_fits ) {
    /* Both fit — higher priority wins */
    if( a.priority > b.priority ) {
      winner = 0;
    } else if( b.priority > a.priority ) {
      winner = 1;
    } else {
      /* Same priority — older wins */
      if( a.age >= b.age ) {
        winner = 0;
      } else {
        winner = 1;
      }
    }
  } else if( a_fits ) {
    winner = 0;
  } else if( b_fits ) {
    winner = 1;
  } else {
    winner = 2;
  }

  return winner;
}

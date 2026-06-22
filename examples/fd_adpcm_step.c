/* fd_adpcm_step — ADPCM encoder step (simplified).
   CHStone benchmark: adpcm (single step).
   Tests: lookup table + if/else + multiply + clamp. */

typedef unsigned int uint;

uint
fd_adpcm_encode( uint sample, uint prev_pred, uint prev_step_idx ) {
  uint step_table[8] = { 7, 8, 9, 10, 11, 12, 14, 16 };
  uint idx = prev_step_idx & 7;
  uint step = step_table[idx];

  uint diff;
  uint sign;
  if( sample >= prev_pred ) {
    diff = sample - prev_pred;
    sign = 0;
  } else {
    diff = prev_pred - sample;
    sign = 8; /* sign bit in 4-bit ADPCM */
  }

  uint delta = 0;
  uint vpdiff = step >> 3;

  if( diff >= step ) {
    delta = delta | 4;
    diff = diff - step;
    vpdiff = vpdiff + step;
  }
  uint half_step = step >> 1;
  if( diff >= half_step ) {
    delta = delta | 2;
    diff = diff - half_step;
    vpdiff = vpdiff + half_step;
  }
  uint quarter_step = step >> 2;
  if( diff >= quarter_step ) {
    delta = delta | 1;
    vpdiff = vpdiff + quarter_step;
  }

  /* Update predictor */
  uint new_pred;
  if( sign == 0 ) {
    new_pred = prev_pred + vpdiff;
  } else {
    if( prev_pred >= vpdiff ) {
      new_pred = prev_pred - vpdiff;
    } else {
      new_pred = 0;
    }
  }

  /* Clamp predictor */
  if( new_pred > 32767 ) new_pred = 32767;

  uint code = sign | delta;
  return (code << 16) | new_pred;
}

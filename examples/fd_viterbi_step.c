/* fd_viterbi_step — Viterbi decoder ACS (Add-Compare-Select) step.
   Tests: array + if/else + multiply-accumulate per state.
   Real-world: error correction in wireless/satellite communications. */

typedef unsigned int uint;

uint
fd_viterbi_acs( uint pm0, uint pm1, uint pm2, uint pm3,
                uint bm00, uint bm01, uint bm10, uint bm11 ) {
  /* State 0: transitions from state 0 (bm00) and state 1 (bm01) */
  uint cand0_0 = pm0 + bm00;
  uint cand0_1 = pm1 + bm01;
  uint new_pm0;
  uint sel0;
  if( cand0_0 <= cand0_1 ) {
    new_pm0 = cand0_0;
    sel0 = 0;
  } else {
    new_pm0 = cand0_1;
    sel0 = 1;
  }

  /* State 1: transitions from state 2 (bm10) and state 3 (bm11) */
  uint cand1_0 = pm2 + bm10;
  uint cand1_1 = pm3 + bm11;
  uint new_pm1;
  uint sel1;
  if( cand1_0 <= cand1_1 ) {
    new_pm1 = cand1_0;
    sel1 = 0;
  } else {
    new_pm1 = cand1_1;
    sel1 = 1;
  }

  /* Pack: sel1[1] | sel0[0] | new_pm1[15:0] | new_pm0[15:0] */
  return (sel1 << 17) | (sel0 << 16) | ((new_pm1 & 0xFF) << 8) | (new_pm0 & 0xFF);
}

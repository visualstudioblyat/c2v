/* fd_levenshtein_row — one row of Levenshtein distance DP (4 positions).
   AoC classic: edit distance, tests min pattern + cascaded deps.
   Competitive metric: pipeline depth vs serial chain. */

typedef unsigned int uint;

uint
fd_lev_row( uint prev0, uint prev1, uint prev2, uint prev3,
            uint diag,  uint match_flags ) {
  uint d0 = 1 - (match_flags & 1);
  uint ins0 = diag + 1;
  uint del0 = prev0 + 1;
  uint sub0 = diag + d0;
  uint cur0 = ins0;
  if( del0 < cur0 ) cur0 = del0;
  if( sub0 < cur0 ) cur0 = sub0;

  uint d1 = 1 - ((match_flags >> 1) & 1);
  uint ins1 = cur0 + 1;
  uint del1 = prev1 + 1;
  uint sub1 = prev0 + d1;
  uint cur1 = ins1;
  if( del1 < cur1 ) cur1 = del1;
  if( sub1 < cur1 ) cur1 = sub1;

  uint d2 = 1 - ((match_flags >> 2) & 1);
  uint ins2 = cur1 + 1;
  uint del2 = prev2 + 1;
  uint sub2 = prev1 + d2;
  uint cur2 = ins2;
  if( del2 < cur2 ) cur2 = del2;
  if( sub2 < cur2 ) cur2 = sub2;

  uint d3 = 1 - ((match_flags >> 3) & 1);
  uint ins3 = cur2 + 1;
  uint del3 = prev3 + 1;
  uint sub3 = prev2 + d3;
  uint cur3 = ins3;
  if( del3 < cur3 ) cur3 = del3;
  if( sub3 < cur3 ) cur3 = sub3;

  return (cur3 << 16) | (cur0 & 0xFFFF);
}

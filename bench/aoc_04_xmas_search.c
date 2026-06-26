/* AoC-style: pattern match 4 adjacent values for a target.
   Like AoC 2024 Day 4 (XMAS search).
   Tests: comparison chain, parallel pattern checks.
   Metric: cycles per 4-element window check. */

typedef unsigned int uint;

uint
aoc_match_pattern( uint a, uint b, uint c, uint d,
                   uint t0, uint t1, uint t2, uint t3 ) {
  uint m0 = (a == t0);
  uint m1 = (b == t1);
  uint m2 = (c == t2);
  uint m3 = (d == t3);
  return m0 & m1 & m2 & m3;
}

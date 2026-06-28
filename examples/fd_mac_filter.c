typedef unsigned int uint;

uint
fd_mac_match( uint mac_hi, uint mac_lo,
              uint filter_hi, uint filter_lo, uint mask_hi, uint mask_lo ) {
  uint match_hi = ((mac_hi ^ filter_hi) & mask_hi) == 0 ? 1 : 0;
  uint match_lo = ((mac_lo ^ filter_lo) & mask_lo) == 0 ? 1 : 0;
  return match_hi & match_lo;
}

uint
fd_mac_filter_4( uint mac_hi, uint mac_lo,
                 uint f0_hi, uint f0_lo,
                 uint f1_hi, uint f1_lo,
                 uint f2_hi, uint f2_lo,
                 uint f3_hi, uint f3_lo ) {
  uint m0 = fd_mac_match(mac_hi, mac_lo, f0_hi, f0_lo, 0xFFFFFFFF, 0xFFFFFFFF);
  uint m1 = fd_mac_match(mac_hi, mac_lo, f1_hi, f1_lo, 0xFFFFFFFF, 0xFFFFFFFF);
  uint m2 = fd_mac_match(mac_hi, mac_lo, f2_hi, f2_lo, 0xFFFFFFFF, 0xFFFFFFFF);
  uint m3 = fd_mac_match(mac_hi, mac_lo, f3_hi, f3_lo, 0xFFFFFFFF, 0xFFFFFFFF);
  return (m3 << 3) | (m2 << 2) | (m1 << 1) | m0;
}

/* fd_tick_to_trade — market data tick processing for HFT.
   Tests: struct + if/else + arithmetic.
   Real-world: trading FPGA (tick-to-trade pipeline). */

typedef unsigned int uint;

typedef struct {
  uint bid;
  uint ask;
  uint last;
  uint volume;
} tick_t;

uint
fd_process_tick( uint bid, uint ask, uint last, uint volume,
                 uint prev_bid, uint prev_ask, uint threshold ) {
  tick_t t;
  t.bid    = bid;
  t.ask    = ask;
  t.last   = last;
  t.volume = volume;

  uint spread = t.ask - t.bid;
  uint mid    = (t.bid + t.ask) >> 1;

  /* Signal: 1=buy, 2=sell, 0=hold */
  uint signal = 0;

  /* Buy if: bid crossed above previous ask AND spread is tight */
  if( t.bid > prev_ask && spread < threshold ) {
    signal = 1;
  }
  /* Sell if: ask crossed below previous bid AND spread is tight */
  if( t.ask < prev_bid && spread < threshold ) {
    signal = 2;
  }

  /* Pack: signal[3:0] | spread[15:0] | mid[15:0] */
  return (signal << 28) | ((spread & 0xFFFF) << 16) | (mid & 0xFFFF);
}

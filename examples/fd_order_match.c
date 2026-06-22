/* fd_order_match — simple order matching engine.
   Tests: struct + if/else chain + priority logic.
   Real-world: exchange matching on FPGA. */

typedef unsigned int uint;

typedef struct {
  uint price;
  uint quantity;
  uint side;  /* 0=buy, 1=sell */
  uint id;
} order_t;

uint
fd_match_orders( uint buy_price, uint buy_qty, uint buy_id,
                 uint sell_price, uint sell_qty, uint sell_id ) {
  order_t buy;
  buy.price    = buy_price;
  buy.quantity = buy_qty;
  buy.side     = 0;
  buy.id       = buy_id;

  order_t sell;
  sell.price    = sell_price;
  sell.quantity = sell_qty;
  sell.side     = 1;
  sell.id       = sell_id;

  /* Match if buy price >= sell price */
  uint matched = 0;
  uint fill_qty = 0;
  uint fill_price = 0;

  if( buy.price >= sell.price ) {
    matched = 1;
    /* Fill at midpoint */
    fill_price = (buy.price + sell.price) >> 1;
    /* Fill quantity is minimum of both */
    if( buy.quantity <= sell.quantity ) {
      fill_qty = buy.quantity;
    } else {
      fill_qty = sell.quantity;
    }
  }

  /* Pack: matched[0] | fill_price[15:0] | fill_qty[15:0] */
  return (matched << 31) | ((fill_price & 0xFFFF) << 16) | (fill_qty & 0xFFFF);
}

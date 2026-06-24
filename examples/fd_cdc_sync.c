/* fd_cdc_sync — clock domain crossing synchronizer logic.
   Tests: shift register pattern + if/else.
   Real-world: multi-clock FPGA designs, CDC safe handshake. */

typedef unsigned int uint;

uint
fd_cdc_handshake( uint req, uint ack_sync2,
                  uint data_in, uint prev_req, uint prev_ack ) {
  /* Detect rising edge of request */
  uint req_edge = req & (~prev_req);

  /* Detect rising edge of synchronized ack */
  uint ack_edge = ack_sync2 & (~prev_ack);

  /* State machine */
  uint new_req;
  uint captured_data;

  if( req_edge ) {
    /* New request: capture data, assert req */
    new_req = 1;
    captured_data = data_in;
  } else if( ack_edge ) {
    /* Ack received: deassert req */
    new_req = 0;
    captured_data = data_in;
  } else {
    new_req = prev_req;
    captured_data = data_in;
  }

  return (new_req << 31) | (captured_data & 0x7FFFFFFF);
}

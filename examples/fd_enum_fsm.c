/* fd_enum_fsm — enum-based state machine.
   Tests: enum constants + switch/case.
   Real-world: protocol state machines, command decoders. */

typedef unsigned int uint;

enum packet_type {
  PKT_DATA   = 0,
  PKT_ACK    = 1,
  PKT_NACK   = 2,
  PKT_RESET  = 3,
  PKT_PING   = 4
};

uint
fd_packet_dispatch( uint pkt_type, uint payload, uint seq ) {
  uint result;
  switch( pkt_type ) {
    case PKT_DATA:
      result = payload ^ seq;
      break;
    case PKT_ACK:
      result = seq + 1;
      break;
    case PKT_NACK:
      result = seq;
      break;
    case PKT_RESET:
      result = 0;
      break;
    case PKT_PING:
      result = payload;
      break;
    default:
      result = 0xDEAD;
      break;
  }
  return result;
}

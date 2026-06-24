/* fd_rate_limiter — token bucket rate limiter.
   Tests: struct pointer + if/else + counter logic.
   Real-world: network QoS, API rate limiting on FPGA. */

typedef unsigned int uint;

typedef struct {
  uint tokens;
  uint max_tokens;
  uint refill_rate;
  uint last_refill;
} rate_limiter_t;

uint
fd_rate_limit_check( rate_limiter_t *rl, uint packet_cost,
                     uint current_time ) {
  /* Refill tokens based on elapsed time */
  uint elapsed = current_time - rl->last_refill;
  uint new_tokens = rl->tokens + elapsed * rl->refill_rate;

  /* Clamp to max */
  if( new_tokens > rl->max_tokens ) {
    new_tokens = rl->max_tokens;
  }

  /* Check if packet is allowed */
  uint allowed;
  if( new_tokens >= packet_cost ) {
    new_tokens = new_tokens - packet_cost;
    allowed = 1;
  } else {
    allowed = 0;
  }

  return (allowed << 31) | (new_tokens & 0x7FFFFFFF);
}

/* fd_pwm — PWM generator with dead-time insertion.
   Tests: comparisons + struct + conditional logic.
   Real-world: motor control, power electronics. */

typedef unsigned int uint;

typedef struct {
  uint counter;
  uint period;
  uint duty;
  uint deadtime;
} pwm_t;

uint
fd_pwm_tick( uint counter, uint period,
             uint duty, uint deadtime ) {
  pwm_t p;
  p.counter = counter;
  p.period = period;
  p.duty = duty;
  p.deadtime = deadtime;

  uint next_counter;
  if( p.counter >= p.period ) {
    next_counter = 0;
  } else {
    next_counter = p.counter + 1;
  }

  /* High-side output: ON when counter < duty */
  uint pwm_h;
  if( p.counter < p.duty ) {
    pwm_h = 1;
  } else {
    pwm_h = 0;
  }

  /* Low-side output: ON when counter >= duty + deadtime */
  uint pwm_l;
  if( p.counter >= p.duty + p.deadtime ) {
    pwm_l = 1;
  } else {
    pwm_l = 0;
  }

  /* Pack: next_counter[15:0] | pwm_h[1] | pwm_l[0] */
  return (next_counter << 2) | (pwm_h << 1) | pwm_l;
}

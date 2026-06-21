/* fd_pid — PID controller.
   Tests: struct state, multiply-accumulate, clamping.
   Real-world: motor control, thermal management. */

typedef unsigned int uint;

typedef struct {
  uint setpoint;
  uint kp;
  uint ki;
  uint kd;
  uint integral;
  uint prev_error;
} pid_t;

uint
fd_pid_step( uint setpoint, uint measured,
             uint kp, uint ki, uint kd,
             uint prev_integral, uint prev_error ) {
  pid_t pid;
  pid.setpoint = setpoint;
  pid.kp = kp;
  pid.ki = ki;
  pid.kd = kd;
  pid.integral = prev_integral;
  pid.prev_error = prev_error;

  uint error;
  if( pid.setpoint >= measured ) {
    error = pid.setpoint - measured;
  } else {
    error = measured - pid.setpoint;
  }

  uint new_integral = pid.integral + error;
  /* Clamp integral to prevent windup */
  if( new_integral > 10000 ) new_integral = 10000;

  uint derivative;
  if( error >= pid.prev_error ) {
    derivative = error - pid.prev_error;
  } else {
    derivative = pid.prev_error - error;
  }

  uint p_term = pid.kp * error;
  uint i_term = pid.ki * new_integral;
  uint d_term = pid.kd * derivative;

  uint output = (p_term + i_term + d_term) >> 8;
  if( output > 4095 ) output = 4095;

  /* Pack output and new state */
  return (output << 16) | (new_integral & 0xFFFF);
}

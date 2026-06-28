typedef unsigned int uint;

int
fd_pid_step( int setpoint, int measured,
             int prev_error, int integral,
             int kp, int ki, int kd ) {
  int error = setpoint - measured;
  int new_integral = integral + error;
  int derivative = error - prev_error;
  int output = kp * error + ki * new_integral + kd * derivative;
  return output >> 8;
}

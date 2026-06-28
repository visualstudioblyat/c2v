typedef unsigned int uint;

int
fd_noise_gate( int sample, int threshold, int attack, int release, int env ) {
  int abs_sample = (sample < 0) ? -sample : sample;
  int new_env;
  if( abs_sample > env ) {
    new_env = env + ((abs_sample - env) * attack >> 8);
  } else {
    new_env = env - ((env - abs_sample) * release >> 8);
  }
  int gain = (new_env > threshold) ? 256 : ((new_env << 8) >> 8);
  return (sample * gain) >> 8;
}

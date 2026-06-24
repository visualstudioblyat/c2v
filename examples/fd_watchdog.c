/* fd_watchdog — hardware watchdog timer logic.
   Tests: counter + comparison + state tracking.
   Real-world: system health monitoring, auto-reset. */

typedef unsigned int uint;

typedef struct {
  uint counter;
  uint timeout;
  uint enabled;
  uint triggered;
} watchdog_t;

uint
fd_watchdog_tick( watchdog_t *wd, uint kick, uint enable ) {
  uint new_counter;
  uint new_triggered;

  if( enable && !wd->enabled ) {
    /* Just enabled — reset counter */
    new_counter = 0;
    new_triggered = 0;
  } else if( kick ) {
    /* Kicked — reset counter */
    new_counter = 0;
    new_triggered = wd->triggered;
  } else if( wd->enabled ) {
    /* Running — increment */
    new_counter = wd->counter + 1;
    if( new_counter >= wd->timeout ) {
      new_triggered = 1;
      new_counter = wd->timeout;
    } else {
      new_triggered = wd->triggered;
    }
  } else {
    new_counter = 0;
    new_triggered = 0;
  }

  return (new_triggered << 31) | (new_counter & 0x7FFFFFFF);
}

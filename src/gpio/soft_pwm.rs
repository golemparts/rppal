// Prevent warning when casting u32 as i64
#![allow(clippy::cast_lossless)]
#![allow(dead_code)]

use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread::{self, sleep};
use std::time::Duration;

use libc::{self, sched_param, timespec, CLOCK_MONOTONIC, PR_SET_TIMERSLACK, SCHED_RR};

use super::{Error, GpioState, Result};

// Only call sleep() if we have enough time remaining
const SLEEP_THRESHOLD: i64 = 250_000;
// Reserve some time for busy waiting
const BUSYWAIT_MAX: i64 = 200_000;
// Subtract from the remaining busy wait time to account for get_time_ns() overhead
const BUSYWAIT_REMAINDER: i64 = 100;

const NANOS_PER_SEC: i64 = 1_000_000_000;

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
enum Msg {
    Reconfigure(Duration, Duration),
    Stop,
}

#[derive(Debug)]
pub(crate) struct SoftPwm {
    pwm_thread: Option<thread::JoinHandle<Result<()>>>,
    sender: Sender<Msg>,
}

impl SoftPwm {
    pub(crate) fn new(
        pin: u8,
        gpio_state: Arc<GpioState>,
        period: Duration,
        pulse_width: Duration,
    ) -> SoftPwm {
        let (sender, receiver): (Sender<Msg>, Receiver<Msg>) = mpsc::channel();

        let pwm_thread = thread::spawn(move || -> Result<()> {
            // Set the scheduling policy to real-time round robin at the highest priority. This
            // will silently fail if we're not running as root.
            #[cfg(target_env = "gnu")]
            let params = sched_param {
                sched_priority: unsafe { libc::sched_get_priority_max(SCHED_RR) },
            };

            #[cfg(target_env = "musl")]
            let params = sched_param {
                sched_priority: unsafe { libc::sched_get_priority_max(SCHED_RR) },
                sched_ss_low_priority: 0,
                sched_ss_repl_period: timespec {
                    tv_sec: 0,
                    tv_nsec: 0,
                },
                sched_ss_init_budget: timespec {
                    tv_sec: 0,
                    tv_nsec: 0,
                },
                sched_ss_max_repl: 0,
            };

            unsafe {
                libc::sched_setscheduler(0, SCHED_RR, &params);
            }

            // Set timer slack to 1 ns (default = 50 µs). This is only relevant if we're unable
            // to set a real-time scheduling policy.
            unsafe {
                libc::prctl(PR_SET_TIMERSLACK, 1);
            }

            let mut period_ns = period.as_nanos() as i64;
            let mut pulse_width_ns = pulse_width.as_nanos() as i64;

            let mut start_ns = get_time_ns();

            loop {
                // PWM active
                if pulse_width_ns > 0 {
                    gpio_state.gpio_mem.set_high(pin);
                }

                // Sleep if we have enough time remaining, while reserving some time
                // for busy waiting to compensate for sleep taking longer than needed.
                if pulse_width_ns >= SLEEP_THRESHOLD {
                    sleep(Duration::from_nanos((pulse_width_ns - BUSYWAIT_MAX) as u64));
                }

                // Busy-wait for the remaining active time, minus BUSYWAIT_REMAINDER
                // to account for get_time_ns() overhead
                loop {
                    if (pulse_width_ns - (get_time_ns() - start_ns)) <= BUSYWAIT_REMAINDER {
                        break;
                    }
                }

                // PWM inactive
                gpio_state.gpio_mem.set_low(pin);

                while let Ok(msg) = receiver.try_recv() {
                    match msg {
                        Msg::Reconfigure(period, pulse_width) => {
                            // Reconfigure period and pulse width
                            pulse_width_ns = pulse_width.as_nanos() as i64;
                            period_ns = period.as_nanos() as i64;

                            if pulse_width_ns > period_ns {
                                pulse_width_ns = period_ns;
                            }
                        }
                        Msg::Stop => {
                            // The main thread asked us to stop
                            return Ok(());
                        }
                    }
                }

                let remaining_ns = period_ns - (get_time_ns() - start_ns);

                // Sleep if we have enough time remaining, while reserving some time
                // for busy waiting to compensate for sleep taking longer than needed.
                if remaining_ns >= SLEEP_THRESHOLD {
                    sleep(Duration::from_nanos((remaining_ns - BUSYWAIT_MAX) as u64));
                }

                // Busy-wait for the remaining inactive time, minus BUSYWAIT_REMAINDER
                // to account for get_time_ns() overhead
                loop {
                    let current_ns = get_time_ns();
                    if (period_ns - (current_ns - start_ns)) <= BUSYWAIT_REMAINDER {
                        start_ns = current_ns;
                        break;
                    }
                }
            }
        });

        SoftPwm {
            pwm_thread: Some(pwm_thread),
            sender,
        }
    }

    pub(crate) fn reconfigure(&mut self, period: Duration, pulse_width: Duration) {
        let _ = self.sender.send(Msg::Reconfigure(period, pulse_width));
    }

    pub(crate) fn stop(&mut self) -> Result<()> {
        let _ = self.sender.send(Msg::Stop);
        if let Some(pwm_thread) = self.pwm_thread.take() {
            match pwm_thread.join() {
                Ok(r) => return r,
                Err(_) => return Err(Error::ThreadPanic),
            }
        }

        Ok(())
    }
}

impl Drop for SoftPwm {
    fn drop(&mut self) {
        // Don't wait for the pwm thread to exit if the main thread is panicking,
        // because we could potentially block indefinitely while unwinding if the
        // pwm thread doesn't respond to the Stop message for some reason.
        if !thread::panicking() {
            let _ = self.stop();
        }
    }
}

// Required because Sender isn't Sync. Implementing Sync for SoftPwm is
// safe because all usage of Sender::send() is locked behind &mut self.
unsafe impl Sync for SoftPwm {}

#[inline(always)]
fn get_time_ns() -> i64 {
    let mut ts = timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };

    unsafe {
        libc::clock_gettime(CLOCK_MONOTONIC, &mut ts);
    }

    (ts.tv_sec as i64 * NANOS_PER_SEC) + ts.tv_nsec as i64
}

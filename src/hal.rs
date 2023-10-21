//! Miscellaneous `embedded-hal` trait implementations.
//!
//! The `hal` module consists of a collection of `embedded-hal` trait
//! implementations for traits that aren't tied to a specific peripheral.
//!
//! This module is only included when either the `hal` or `hal-unproven` feature
//! flag is enabled.

use std::time::{Duration, Instant};

use embedded_hal::delay::DelayUs;
use spin_sleep::sleep;
use void::Void;

/// Implements the `embedded-hal` `DelayMs` and `DelayUs` traits.
#[derive(Debug, Default)]
pub struct Delay;

/// `Delay` trait implementation for `embedded-hal` v1.0.0-alpha.9.
impl Delay {
    /// Constructs a new `Delay`.
    pub fn new() -> Delay {
        Delay {}
    }
}

/// `DelayMs<u8>` trait implementation for `embedded-hal` v0.2.7.
impl embedded_hal_0::blocking::delay::DelayMs<u8> for Delay {
    fn delay_ms(&mut self, ms: u8) {
        DelayUs::delay_ms(self, ms as u32);
    }
}

/// `DelayMs<u16>` trait implementation for `embedded-hal` v0.2.7.
impl embedded_hal_0::blocking::delay::DelayMs<u16> for Delay {
    fn delay_ms(&mut self, ms: u16) {
        DelayUs::delay_ms(self, ms as u32);
    }
}

/// `DelayMs<u32>` trait implementation for `embedded-hal` v0.2.7.
impl embedded_hal_0::blocking::delay::DelayMs<u32> for Delay {
    fn delay_ms(&mut self, ms: u32) {
        DelayUs::delay_ms(self, ms);
    }
}

/// `DelayMs<u64>` trait implementation for `embedded-hal` v0.2.7.
impl embedded_hal_0::blocking::delay::DelayMs<u64> for Delay {
    fn delay_ms(&mut self, mut ms: u64) {
        while ms > (u32::MAX as u64) {
            ms -= u32::MAX as u64;
            DelayUs::delay_ms(self, u32::MAX);
        }

        DelayUs::delay_ms(self, ms as u32);
    }
}

/// `DelayUs<u8>` trait implementation for `embedded-hal` v0.2.7.
impl embedded_hal_0::blocking::delay::DelayUs<u8> for Delay {
    fn delay_us(&mut self, us: u8) {
        DelayUs::delay_us(self, us as u32);
    }
}

/// `DelayUs<u16>` trait implementation for `embedded-hal` v0.2.7.
impl embedded_hal_0::blocking::delay::DelayUs<u16> for Delay {
    fn delay_us(&mut self, us: u16) {
        DelayUs::delay_us(self, us as u32);
    }
}

/// `DelayUs` trait implementation for `embedded-hal` v1.0.0-alpha.9.
impl DelayUs for Delay {
    fn delay_us(&mut self, us: u32) {
        sleep(Duration::from_micros(us.into()));
    }

    fn delay_ms(&mut self, ms: u32) {
        sleep(Duration::from_millis(u64::from(ms)));
    }
}

/// `DelayUs<u32>` trait implementation for `embedded-hal` v0.2.7.
impl embedded_hal_0::blocking::delay::DelayUs<u32> for Delay {
    fn delay_us(&mut self, us: u32) {
        DelayUs::delay_us(self, us);
    }
}

/// `DelayUs<u64>` trait implementation for `embedded-hal` v0.2.7.
impl embedded_hal_0::blocking::delay::DelayUs<u64> for Delay {
    fn delay_us(&mut self, mut us: u64) {
        while us > (u32::MAX as u64) {
            us -= u32::MAX as u64;
            DelayUs::delay_us(self, u32::MAX);
        }

        DelayUs::delay_us(self, us as u32);
    }
}

/// Newtype wrapper for `f64`. Converts into `Duration`.
pub struct Hertz(pub f64);

const MICROS_PER_SEC: f64 = 1_000_000.0;

impl From<Hertz> for Duration {
    fn from(item: Hertz) -> Self {
        if item.0 > 0.0 && item.0.is_finite() {
            Duration::from_micros(((1.0 / item.0) * MICROS_PER_SEC) as u64)
        } else {
            Duration::default()
        }
    }
}

/// Implements the `embedded-hal` `CountDown` trait.
#[derive(Debug, Copy, Clone)]
pub struct Timer {
    start: Instant,
    duration: Duration,
}

impl Timer {
    /// Constructs a new `Timer`.
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            duration: Duration::from_micros(0),
        }
    }
}

impl Default for Timer {
    fn default() -> Self {
        Timer::new()
    }
}

/// `CountDown` trait implementation for `embedded-hal` v0.2.7.
impl embedded_hal_0::timer::CountDown for Timer {
    type Time = Duration;

    /// Starts the timer with a `timeout`.
    fn start<T>(&mut self, timeout: T)
    where
        T: Into<Self::Time>,
    {
        self.start = Instant::now();
        self.duration = timeout.into();
    }

    /// Returns `Ok` if the timer has wrapped.
    fn wait(&mut self) -> nb::Result<(), Void> {
        if self.start.elapsed() >= self.duration {
            Ok(())
        } else {
            Err(nb::Error::WouldBlock)
        }
    }
}

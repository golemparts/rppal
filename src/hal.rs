//! Miscellaneous `embedded-hal` trait implementations.
//!
//! The `hal` module consists of a collection of `embedded-hal` trait
//! implementations for traits that aren't tied to a specific peripheral.
//!
//! This module is only included when either the `hal` or `hal-unproven` feature
//! flag is enabled.

use std::time::Duration;
#[cfg(feature = "embedded-hal-0")]
use std::time::Instant;

/// Implements the `embedded-hal` `DelayMs` and `DelayNs` traits.
#[derive(Debug, Default)]
pub struct Delay;

impl Delay {
    /// Constructs a new `Delay`.
    pub fn new() -> Delay {
        Delay {}
    }
}

#[cfg(feature = "embedded-hal-0")]
impl embedded_hal_0::blocking::delay::DelayMs<u8> for Delay {
    fn delay_ms(&mut self, ms: u8) {
        embedded_hal::delay::DelayNs::delay_ms(self, ms as u32);
    }
}

#[cfg(feature = "embedded-hal-0")]
impl embedded_hal_0::blocking::delay::DelayMs<u16> for Delay {
    fn delay_ms(&mut self, ms: u16) {
        embedded_hal::delay::DelayNs::delay_ms(self, ms as u32);
    }
}

#[cfg(feature = "embedded-hal-0")]
impl embedded_hal_0::blocking::delay::DelayMs<u32> for Delay {
    fn delay_ms(&mut self, ms: u32) {
        embedded_hal::delay::DelayNs::delay_ms(self, ms);
    }
}

#[cfg(feature = "embedded-hal-0")]
impl embedded_hal_0::blocking::delay::DelayMs<u64> for Delay {
    fn delay_ms(&mut self, mut ms: u64) {
        while ms > (u32::MAX as u64) {
            ms -= u32::MAX as u64;
            embedded_hal::delay::DelayNs::delay_ms(self, u32::MAX);
        }

        embedded_hal::delay::DelayNs::delay_ms(self, ms as u32);
    }
}

#[cfg(feature = "embedded-hal-0")]
impl embedded_hal_0::blocking::delay::DelayUs<u8> for Delay {
    fn delay_us(&mut self, us: u8) {
        embedded_hal::delay::DelayNs::delay_us(self, us as u32);
    }
}

#[cfg(feature = "embedded-hal-0")]
impl embedded_hal_0::blocking::delay::DelayUs<u16> for Delay {
    fn delay_us(&mut self, us: u16) {
        embedded_hal::delay::DelayNs::delay_us(self, us as u32);
    }
}

#[cfg(feature = "embedded-hal")]
impl embedded_hal::delay::DelayNs for Delay {
    fn delay_ns(&mut self, ns: u32) {
        spin_sleep::sleep(Duration::from_nanos(ns.into()));
    }

    fn delay_us(&mut self, us: u32) {
        spin_sleep::sleep(Duration::from_micros(us.into()));
    }

    fn delay_ms(&mut self, ms: u32) {
        spin_sleep::sleep(Duration::from_millis(ms.into()));
    }
}

#[cfg(feature = "embedded-hal-0")]
impl embedded_hal_0::blocking::delay::DelayUs<u32> for Delay {
    fn delay_us(&mut self, us: u32) {
        embedded_hal::delay::DelayNs::delay_us(self, us);
    }
}

#[cfg(feature = "embedded-hal-0")]
impl embedded_hal_0::blocking::delay::DelayUs<u64> for Delay {
    fn delay_us(&mut self, mut us: u64) {
        while us > (u32::MAX as u64) {
            us -= u32::MAX as u64;
            embedded_hal::delay::DelayNs::delay_us(self, u32::MAX);
        }

        embedded_hal::delay::DelayNs::delay_us(self, us as u32);
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
#[cfg(feature = "embedded-hal-0")]
#[derive(Debug, Copy, Clone)]
pub struct Timer {
    start: Instant,
    duration: Duration,
}

#[cfg(feature = "embedded-hal-0")]
impl Timer {
    /// Constructs a new `Timer`.
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            duration: Duration::from_micros(0),
        }
    }
}

#[cfg(feature = "embedded-hal-0")]
impl Default for Timer {
    fn default() -> Self {
        Timer::new()
    }
}

#[cfg(feature = "embedded-hal-0")]
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
    fn wait(&mut self) -> nb::Result<(), void::Void> {
        if self.start.elapsed() >= self.duration {
            Ok(())
        } else {
            Err(nb::Error::WouldBlock)
        }
    }
}

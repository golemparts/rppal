// Copyright (c) 2017-2021 Rene van der Meer
//
// Permission is hereby granted, free of charge, to any person obtaining a
// copy of this software and associated documentation files (the "Software"),
// to deal in the Software without restriction, including without limitation
// the rights to use, copy, modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software, and to permit persons to whom the
// Software is furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL
// THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
// FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.

//! Miscellaneous `embedded-hal` trait implementations.
//!
//! The `hal` module consists of a collection of `embedded-hal` trait
//! implementations for traits that aren't tied to a specific peripheral.
//!
//! This module is only included when either the `hal` or `hal-unproven` feature
//! flag is enabled.

use core::convert::Infallible;
use std::time::{Duration, Instant};

use embedded_hal::blocking::delay::{DelayMs, DelayUs};
use embedded_hal::timer::CountDown;
use spin_sleep::sleep;
use void::Void;

/// Implements the `embedded-hal` `DelayMs` and `DelayUs` traits.
#[derive(Debug, Default)]
pub struct Delay {}

impl Delay {
    /// Constructs a new `Delay`.
    pub fn new() -> Delay {
        Delay {}
    }
}

impl DelayMs<u8> for Delay {
    type Error = Infallible;

    fn try_delay_ms(&mut self, ms: u8) -> Result<(), Self::Error> {
        sleep(Duration::from_millis(u64::from(ms)));
        Ok(())
    }
}

impl embedded_hal_0::blocking::delay::DelayMs<u8> for Delay {
    fn delay_ms(&mut self, ms: u8) {
        self.try_delay_ms(ms).unwrap()
    }
}

impl DelayMs<u16> for Delay {
    type Error = Infallible;

    fn try_delay_ms(&mut self, ms: u16) -> Result<(), Self::Error> {
        sleep(Duration::from_millis(u64::from(ms)));
        Ok(())
    }
}

impl embedded_hal_0::blocking::delay::DelayMs<u16> for Delay {
    fn delay_ms(&mut self, ms: u16) {
        self.try_delay_ms(ms).unwrap()
    }
}

impl DelayMs<u32> for Delay {
    type Error = Infallible;

    fn try_delay_ms(&mut self, ms: u32) -> Result<(), Self::Error> {
        sleep(Duration::from_millis(u64::from(ms)));
        Ok(())
    }
}

impl embedded_hal_0::blocking::delay::DelayMs<u32> for Delay {
    fn delay_ms(&mut self, ms: u32) {
        self.try_delay_ms(ms).unwrap()
    }
}

impl DelayMs<u64> for Delay {
    type Error = Infallible;

    fn try_delay_ms(&mut self, ms: u64) -> Result<(), Self::Error> {
        sleep(Duration::from_millis(ms));
        Ok(())
    }
}

impl embedded_hal_0::blocking::delay::DelayMs<u64> for Delay {
    fn delay_ms(&mut self, ms: u64) {
        self.try_delay_ms(ms).unwrap()
    }
}

impl DelayUs<u8> for Delay {
    type Error = Infallible;

    fn try_delay_us(&mut self, us: u8) -> Result<(), Self::Error> {
        sleep(Duration::from_micros(us.into()));
        Ok(())
    }
}

impl embedded_hal_0::blocking::delay::DelayUs<u8> for Delay {
    fn delay_us(&mut self, us: u8) {
        self.try_delay_us(us).unwrap()
    }
}

impl DelayUs<u16> for Delay {
    type Error = Infallible;

    fn try_delay_us(&mut self, us: u16) -> Result<(), Self::Error> {
        sleep(Duration::from_micros(us.into()));
        Ok(())
    }
}

impl embedded_hal_0::blocking::delay::DelayUs<u16> for Delay {
    fn delay_us(&mut self, us: u16) {
        self.try_delay_us(us).unwrap()
    }
}

impl DelayUs<u32> for Delay {
    type Error = Infallible;

    fn try_delay_us(&mut self, us: u32) -> Result<(), Self::Error> {
        sleep(Duration::from_micros(us.into()));
        Ok(())
    }
}

impl embedded_hal_0::blocking::delay::DelayUs<u32> for Delay {
    fn delay_us(&mut self, us: u32) {
        self.try_delay_us(us).unwrap()
    }
}

impl DelayUs<u64> for Delay {
    type Error = Infallible;

    fn try_delay_us(&mut self, us: u64) -> Result<(), Self::Error> {
        sleep(Duration::from_micros(us));
        Ok(())
    }
}

impl embedded_hal_0::blocking::delay::DelayUs<u64> for Delay {
    fn delay_us(&mut self, us: u64) {
        self.try_delay_us(us).unwrap()
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

impl CountDown for Timer {
    type Time = Duration;
    type Error = Infallible;

    /// Starts the timer with a `timeout`.
    fn try_start<T>(&mut self, timeout: T) -> Result<(), Self::Error>
    where
        T: Into<Self::Time>,
    {
        self.start = Instant::now();
        self.duration = timeout.into();
        Ok(())
    }

    /// Returns `Ok` if the timer has wrapped.
    fn try_wait(&mut self) -> nb::Result<(), Self::Error> {
        if self.start.elapsed() >= self.duration {
            Ok(())
        } else {
            Err(nb::Error::WouldBlock)
        }
    }
}

impl embedded_hal_0::timer::CountDown for Timer {
    type Time = Duration;

    /// Starts the timer with a `timeout`.
    fn start<T>(&mut self, timeout: T)
    where
        T: Into<Self::Time>,
    {
        self.try_start(timeout).unwrap()
    }

    /// Returns `Ok` if the timer has wrapped.
    fn wait(&mut self) -> nb::Result<(), Void> {
        Ok(self.try_wait().unwrap())
    }
}

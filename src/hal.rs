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

use std::time::{Duration, Instant};
use core::convert::Infallible;

use embedded_hal::timer::CountDown;

pub use linux_embedded_hal::Delay;

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

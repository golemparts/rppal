// Copyright (c) 2017-2018 Rene van der Meer
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

//! Interface for the PWM peripheral.
//!
//! RPPAL configures the Raspberry Pi's PWM peripheral through the `/sys/class/pwm`
//! sysfs interface.
//!
//! ## PWM channels
//!
//! The BCM283x SoC supports two hardware PWM channels. By default, both channels
//! are disabled.
//!
//! Note: Overlapping pins with SPI/I2C
//!
//! ## Using PWM without superuser privileges (`sudo`)
//!
//! As of kernel version 4.14.34, released on April 16 2018, it's possible to
//! configure your Raspberry Pi to allow non-root access to PWM. 4.14.34 includes
//! a [patch] that allows udev to change file permissions when a
//! PWM channel is exported. This will let any user that's a member of the `gpio`
//! group configure PWM without having to use `sudo`.
//!
//! The udev rules needed to make this work haven't been patched in yet as of
//! June 2018, but you can easily add them yourself. Make sure you're running
//! 4.14.34 or later, and append the following snippet to
//! `/etc/udev/rules.d/99-com.rules`. Reboot the Raspberry Pi afterwards.
//!
//! ```text
//! SUBSYSTEM=="pwm*", PROGRAM="/bin/sh -c '\
//!     chown -R root:gpio /sys/class/pwm && chmod -R 770 /sys/class/pwm;\
//!     chown -R root:gpio /sys/devices/platform/soc/*.pwm/pwm/pwmchip* &&\
//!     chmod -R 770 /sys/devices/platform/soc/*.pwm/pwm/pwmchip*\
//! '"
//! ```
//!
//! ## Troubleshooting
//!
//! ### Permission denied
//!
//! If [`new`] returns an `io::ErrorKind::PermissionDenied`
//! error, make sure `/sys/class/pwm` and all of its subdirectories
//! are owned by `root:gpio`, the current user is a member of the `gpio` group
//! and udev is properly configured as mentioned above. Alternatively, you can
//! launch your application using `sudo`.
//!
//! [patch]: https://github.com/raspberrypi/linux/issues/1983

use std::io;
use std::result;
use std::time::Duration;

mod sysfs;

quick_error! {
/// Errors that can occur when accessing the PWM peripheral.
    #[derive(Debug)]
    pub enum Error {
/// IO error.
        Io(err: io::Error) { description(err.description()) from() }
    }
}

/// Result type returned from methods that can have `pwm::Error`s.
pub type Result<T> = result::Result<T, Error>;

/// Channel
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Channel {
    Pwm0 = 0,
    Pwm1 = 1,
}

/// Polarity
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Polarity {
    Normal,
    Inverse,
}

/// Provides access to the Raspberry Pi's PWM peripheral.
///
/// Before using `Pwm`, make sure your Raspberry Pi has the necessary PWM
/// channels enabled. More information can be found [here].
///
/// [here]: index.html
pub struct Pwm {
    channel: Channel,
}

impl Pwm {
    /// Constructs a new `Pwm`.
    pub fn new(channel: Channel) -> Result<Pwm> {
        sysfs::export(channel as u8)?;

        let pwm = Pwm { channel };

        // Always reset "enable" to 0. The sysfs interface has a bug where a previous
        // export may have left "enable" as 1 after unexporting. On the next export,
        // "enable" is still set to 1, even though the channel isn't enabled.
        let _ = pwm.disable();

        // Default settings
        let _ = pwm.set_duty_cycle(Duration::from_secs(0));
        let _ = pwm.set_period(Duration::from_secs(0));
        let _ = pwm.set_polarity(Polarity::Normal);

        Ok(pwm)
    }

    /// Constructs a new `Pwm` using the specified settings.
    pub fn with_settings(
        channel: Channel,
        period: Duration,
        duty_cycle: Duration,
        polarity: Polarity,
        enabled: bool,
    ) -> Result<Pwm> {
        sysfs::export(channel as u8)?;

        let pwm = Pwm { channel };

        // Always reset "enable" to 0. The sysfs pwm interface has a bug where a previous
        // export may have left "enable" as 1 after unexporting. On the next export,
        // "enable" is still set to 1, even though the channel isn't enabled.
        let _ = pwm.disable();

        // Set duty cycle to 0 first in case the new period is shorter than the current duty cycle
        let _ = pwm.set_duty_cycle(Duration::from_secs(0));

        pwm.set_period(period)?;
        pwm.set_duty_cycle(duty_cycle)?;
        pwm.set_polarity(polarity)?;
        if enabled {
            pwm.enable()?;
        }

        Ok(pwm)
    }

    // Gets the period.
    pub fn period(&self) -> Result<Duration> {
        Ok(Duration::from_nanos(sysfs::period(self.channel as u8)?))
    }

    /// Sets the period.
    ///
    /// `period` must be longer than or equal to the selected duty cycle.
    pub fn set_period(&self, period: Duration) -> Result<()> {
        sysfs::set_period(
            self.channel as u8,
            u64::from(period.subsec_nanos())
                .saturating_add(period.as_secs().saturating_mul(1_000_000_000)),
        )?;

        Ok(())
    }

    /// Gets the duty cycle.
    pub fn duty_cycle(&self) -> Result<Duration> {
        Ok(Duration::from_nanos(sysfs::duty_cycle(self.channel as u8)?))
    }

    /// Sets the duty cycle.
    ///
    /// `duty_cycle` must be shorter than or equal to the selected period.
    pub fn set_duty_cycle(&self, duty_cycle: Duration) -> Result<()> {
        sysfs::set_duty_cycle(
            self.channel as u8,
            u64::from(duty_cycle.subsec_nanos())
                .saturating_add(duty_cycle.as_secs().saturating_mul(1_000_000_000)),
        )?;

        Ok(())
    }

    /// Gets the polarity.
    pub fn polarity(&self) -> Result<Polarity> {
        Ok(sysfs::polarity(self.channel as u8)?)
    }

    /// Sets the polarity.
    ///
    /// Changing the polarity from [`Normal`] to [`Inverse`] inverts
    /// the selected duty cycle.
    ///
    /// By default, `polarity` is set to [`Normal`].
    ///
    /// [`Normal`]: enum.Polarity.html
    pub fn set_polarity(&self, polarity: Polarity) -> Result<()> {
        sysfs::set_polarity(self.channel as u8, polarity)?;

        Ok(())
    }

    /// Gets the enabled status.
    pub fn enabled(&self) -> Result<bool> {
        Ok(sysfs::enabled(self.channel as u8)?)
    }

    /// Enables the PWM channel.
    pub fn enable(&self) -> Result<()> {
        sysfs::set_enabled(self.channel as u8, true)?;

        Ok(())
    }

    /// Disables the PWM channel.
    pub fn disable(&self) -> Result<()> {
        sysfs::set_enabled(self.channel as u8, false)?;

        Ok(())
    }
}

impl Drop for Pwm {
    fn drop(&mut self) {
        let _ = sysfs::set_enabled(self.channel as u8, false);
        let _ = sysfs::unexport(self.channel as u8);
    }
}

// Copyright (c) 2017-2019 Rene van der Meer
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
//! RPPAL controls the Raspberry Pi's PWM peripheral through the `/sys/class/pwm` sysfs
//! interface.
//!
//! ## PWM channels
//!
//! The BCM283x SoC supports two hardware PWM channels. By default, both channels
//! are disabled. To enable only PWM0 on its default pin (BCM GPIO 18, physical pin 12),
//! add `dtoverlay=pwm` to `/boot/config.txt`. If you need both PWM channels, replace
//! `pwm` with `pwm-2chan`, which enables PWM0 on BCM GPIO 18 (physical pin 12), and PWM1
//! on BCM GPIO 19 (physical pin 35). More details on enabling and configuring PWM on
//! other GPIO pins than the default ones can be found in `/boot/overlays/README`.
//!
//! The Raspberry Pi's analog audio output needs both PWM channels. Using both audio out
//! and PWM at the same time may cause issues.
//!
//! Some of the GPIO pins capable of supporting hardware PWM can also be configured for
//! use with other peripherals. Be careful not to enable two peripherals on the same pin
//! at the same time.
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
//! ### Not found
//!
//! If [`new`] returns an `io::ErrorKind::NotFound` error, you may have
//! forgotten to enable the selected PWM channel. The configuration options
//! to enable either of the two PWM channels are listed above.
//!
//! [patch]: https://github.com/raspberrypi/linux/issues/1983
//! [`new`]: struct.Pwm.html#method.new

use std::error;
use std::fmt;
use std::io;
use std::result;
use std::time::Duration;

mod sysfs;

/// Errors that can occur when accessing the PWM peripheral.
#[derive(Debug)]
pub enum Error {
    /// IO error.
    Io(io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Error::Io(ref err) => write!(f, "IO error: {}", err),
        }
    }
}

impl error::Error for Error {}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}

/// Result type returned from methods that can have `pwm::Error`s.
pub type Result<T> = result::Result<T, Error>;

/// PWM channels.
///
/// More information on enabling and configuring the PWM channels can be
/// found [here].
///
/// [here]: index.html
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Channel {
    Pwm0 = 0,
    Pwm1 = 1,
}

/// Output polarities.
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Polarity {
    Normal,
    Inverse,
}

/// Provides access to the Raspberry Pi's PWM peripheral.
///
/// Before using `Pwm`, make sure the selected PWM channel has been configured
/// and activated. More information can be found [here].
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
        let _ = pwm.set_pulse_width(Duration::from_secs(0));
        let _ = pwm.set_period(Duration::from_secs(0));
        let _ = pwm.set_polarity(Polarity::Normal);

        Ok(pwm)
    }

    /// Constructs a new `Pwm` using the specified settings.
    ///
    /// `period` represents the time it takes for the PWM channel to complete one cycle.
    ///
    /// `pulse_width` represents the amount of time the PWM channel's logic level is set
    /// high (or low if the polarity is set to [`Inverse`]) during a single period.
    ///
    /// `polarity` configures the active logic level as either high ([`Normal`]) or low ([`Inverse`]).
    ///
    /// `enabled` immediately enables PWM on the selected channel.
    ///
    /// This method will fail if `period` is shorter than `duty_cycle`.
    ///
    /// [`Normal`]: enum.Polarity.html
    /// [`Inverse`]: enum.Polarity.html
    pub fn with_period(
        channel: Channel,
        period: Duration,
        pulse_width: Duration,
        polarity: Polarity,
        enabled: bool,
    ) -> Result<Pwm> {
        sysfs::export(channel as u8)?;

        let pwm = Pwm { channel };

        // Always reset "enable" to 0. The sysfs pwm interface has a bug where a previous
        // export may have left "enable" as 1 after unexporting. On the next export,
        // "enable" is still set to 1, even though the channel isn't enabled.
        let _ = pwm.disable();

        // Set pulse width to 0 first in case the new period is shorter than the current pulse width
        let _ = sysfs::set_pulse_width(channel as u8, 0);

        pwm.set_period(period)?;
        pwm.set_pulse_width(pulse_width)?;
        pwm.set_polarity(polarity)?;
        if enabled {
            pwm.enable()?;
        }

        Ok(pwm)
    }

    /// Constructs a new `Pwm` using the specified settings.
    ///
    /// `with_frequency` is a convenience method that converts `frequency` to a period,
    /// and calculates the duty cycle as a percentage of the frequency.
    ///
    /// `frequency` is specified in herz (Hz).
    ///
    /// `duty_cycle` is specified as a floating point ratio between `0.0` (0%) and `1.0` (100%).
    ///
    /// `polarity` configures the active logic level as either high ([`Normal`]) or low ([`Inverse`]).
    ///
    /// `enabled` immediately enables PWM on the selected channel.
    ///
    /// [`Normal`]: enum.Polarity.html
    /// [`Inverse`]: enum.Polarity.html
    pub fn with_frequency(
        channel: Channel,
        frequency: f64,
        duty_cycle: f64,
        polarity: Polarity,
        enabled: bool,
    ) -> Result<Pwm> {
        sysfs::export(channel as u8)?;

        let pwm = Pwm { channel };

        // Always reset "enable" to 0. The sysfs pwm interface has a bug where a previous
        // export may have left "enable" as 1 after unexporting. On the next export,
        // "enable" is still set to 1, even though the channel isn't enabled.
        let _ = pwm.disable();

        // Set pulse width to 0 first in case the new period is shorter than the current pulse width
        let _ = sysfs::set_pulse_width(channel as u8, 0);

        // Convert to nanoseconds
        let period = (1.0f64 / frequency) * 1_000_000_000f64;
        let pulse_width = period * duty_cycle.max(0.0).min(1.0);

        sysfs::set_period(channel as u8, period as u64)?;
        sysfs::set_pulse_width(channel as u8, pulse_width as u64)?;

        pwm.set_polarity(polarity)?;
        if enabled {
            pwm.enable()?;
        }

        Ok(pwm)
    }

    /// Returns the configured period.
    pub fn period(&self) -> Result<Duration> {
        Ok(Duration::from_nanos(sysfs::period(self.channel as u8)?))
    }

    /// Sets the period.
    ///
    /// `period` represents the time it takes for the PWM channel to complete one cycle.
    /// The specified period must be longer than or equal to the pulse width.
    pub fn set_period(&self, period: Duration) -> Result<()> {
        sysfs::set_period(
            self.channel as u8,
            u64::from(period.subsec_nanos())
                .saturating_add(period.as_secs().saturating_mul(1_000_000_000)),
        )?;

        Ok(())
    }

    /// Returns the configured pulse width.
    pub fn pulse_width(&self) -> Result<Duration> {
        Ok(Duration::from_nanos(sysfs::pulse_width(
            self.channel as u8,
        )?))
    }

    /// Sets the pulse width.
    ///
    /// `pulse_width` represents the amount of time the PWM channel's logic level is set
    /// high (or low if the polarity is set to [`Inverse`]) during a single period. The
    /// pulse width must be shorter than or equal to the period.
    ///
    /// [`Inverse`]: enum.Polarity.html
    pub fn set_pulse_width(&self, pulse_width: Duration) -> Result<()> {
        sysfs::set_pulse_width(
            self.channel as u8,
            u64::from(pulse_width.subsec_nanos())
                .saturating_add(pulse_width.as_secs().saturating_mul(1_000_000_000)),
        )?;

        Ok(())
    }

    /// Returns the configured duty cycle.
    ///
    /// `duty_cycle` is a convenience method that calculates the duty cycle as a
    /// floating point ratio between `0.0` (0%) and `1.0` (100%) based on the configured
    /// period and pulse width.
    pub fn duty_cycle(&self) -> Result<f64> {
        let period = sysfs::period(self.channel as u8)? as f64;
        let pulse_width = sysfs::pulse_width(self.channel as u8)? as f64;

        Ok((pulse_width / period).max(0.0).min(1.0))
    }

    /// Sets the duty cycle.
    ///
    /// `duty_cycle` represents the amount of time the PWM channel's logic level is set
    /// high (or low if the polarity is set to [`Inverse`]) during a single period. The
    /// duty cycle is specified as a floating point ratio between `0.0` (0%) and `1.0` (100%).
    ///
    /// `set_duty_cycle` is a convenience method that converts `duty_cycle` to the
    /// correct pulse width value, based on the configured period.
    ///
    /// [`Inverse`]: enum.Polarity.html
    pub fn set_duty_cycle(&self, duty_cycle: f64) -> Result<()> {
        let period = sysfs::period(self.channel as u8)? as f64;
        let pulse_width = period * duty_cycle.max(0.0).min(1.0);

        sysfs::set_pulse_width(self.channel as u8, pulse_width as u64)?;

        Ok(())
    }

    /// Sets the frequency and duty cycle.
    ///
    /// `set_frequency` is a convenience method that converts `frequency` to a period,
    /// and calculates the duty cycle as a percentage of the frequency.
    ///
    /// `frequency` is specified in herz (Hz).
    ///
    /// `duty_cycle` is specified as a floating point ratio between `0.0` (0%) and `1.0` (100%).
    pub fn set_frequency(&self, frequency: f64, duty_cycle: f64) -> Result<()> {
        // Set duty cycle to 0 first in case the new period is shorter than the current duty cycle
        let _ = sysfs::set_pulse_width(self.channel as u8, 0);

        // Convert to nanoseconds
        let period = (1.0f64 / frequency) * 1_000_000_000f64;
        let pulse_width = period * duty_cycle.max(0.0).min(1.0);

        sysfs::set_period(self.channel as u8, period as u64)?;
        sysfs::set_pulse_width(self.channel as u8, pulse_width as u64)?;

        Ok(())
    }

    /// Returns the configured polarity.
    pub fn polarity(&self) -> Result<Polarity> {
        Ok(sysfs::polarity(self.channel as u8)?)
    }

    /// Sets the polarity.
    ///
    /// `polarity` configures the active logic level as either high ([`Normal`]) or low ([`Inverse`]).
    ///
    /// By default, `polarity` is set to [`Normal`].
    ///
    /// [`Normal`]: enum.Polarity.html
    /// [`Inverse`]: enum.Polarity.html
    pub fn set_polarity(&self, polarity: Polarity) -> Result<()> {
        sysfs::set_polarity(self.channel as u8, polarity)?;

        Ok(())
    }

    /// Checks whether PWM is currently enabled on the selected channel.
    pub fn enabled(&self) -> Result<bool> {
        Ok(sysfs::enabled(self.channel as u8)?)
    }

    /// Enables PWM on the selected channel.
    pub fn enable(&self) -> Result<()> {
        sysfs::set_enabled(self.channel as u8, true)?;

        Ok(())
    }

    /// Disables PWM on the selected channel.
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

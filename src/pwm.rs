//! Interface for the PWM peripheral.
//!
//! RPPAL controls the Raspberry Pi's PWM peripheral through the `pwm` sysfs
//! interface.
//!
//! ## PWM channels
//!
//! ### Older models (older than Raspberry Pi 5)
//!
//! The BCM283x SoC supports 2 hardware PWM channels. By default, the channels are
//! mapped as follows:
//!
//! * PWM0 = GPIO12/GPIO18
//! * PWM1 = GPIO13/GPIO19
//!
//! Consult the official documentation on how to enable and configure these.
//!
//! The Raspberry Pi's analog audio output uses both PWM channels. Playing audio and
//! simultaneously accessing a PWM channel may cause issues.
//!
//! Some of the GPIO pins capable of supporting hardware PWM can also be configured for
//! use with other peripherals. Be careful not to enable two peripherals on the same pin
//! at the same time.
//!
//! ### Newer models (Raspberry Pi 5 and later)
//!
//! The Raspberry Pi 5 and other recent models support 4 hardware PWM channels. By
//! default, the channels are mapped as follows:
//!
//! * PWM0 = GPIO12
//! * PWM1 = GPIO13
//! * PWM2 = GPIO18
//! * PWM3 = GPIO19
//!
//! Consult the official documentation on how to enable and configure these.
//!
//! Some of the GPIO pins capable of supporting hardware PWM can also be configured for
//! use with other peripherals. Be careful not to enable two peripherals on the same pin
//! at the same time.
//!
//! ## Troubleshooting
//!
//! ### Permission denied
//!
//! If [`new`] returns an `io::ErrorKind::PermissionDenied` error, make sure
//! `/sys/class/pwm` and its subdirectories has the appropriate permissions for the current user.
//! Alternatively, you can launch your application using `sudo`.
//!
//! ### Not found
//!
//! If [`new`] returns an `io::ErrorKind::NotFound` error, you may have
//! forgotten to enable the selected PWM channel. Consult the official Raspberry Pi documentation
//! to correctly configure the needed channel(s).
//!
//! [`new`]: struct.Pwm.html#method.new

use std::error;
use std::fmt;
use std::io;
use std::result;
use std::time::Duration;

#[cfg(any(
    feature = "embedded-hal-0",
    feature = "embedded-hal",
    feature = "embedded-hal-nb"
))]
mod hal;
#[cfg(feature = "hal-unproven")]
mod hal_unproven;
mod sysfs;

use crate::system::DeviceInfo;

const NANOS_PER_SEC: f64 = 1_000_000_000.0;

/// Errors that can occur when accessing the PWM peripheral.
#[derive(Debug)]
pub enum Error {
    /// I/O error.
    Io(io::Error),
    /// Unknown model.
    ///
    /// The Raspberry Pi model or SoC can't be identified. Support for
    /// new models is usually added shortly after they are officially
    /// announced and available to the public. Make sure you're using
    /// the latest release of RPPAL.
    ///
    /// You may also encounter this error if your Linux distribution
    /// doesn't provide any of the common user-accessible system files
    /// that are used to identify the model and SoC.
    UnknownModel,
    /// Invalid channel.
    ///
    /// The selected PWM channel is not available on this device. You may
    /// encounter this error if the Raspberry Pi model only has a limited
    /// number of channels, the selected channel hasn't been properly
    /// configured in `/boot/firmware/config.txt`, or the channel isn't
    /// supported by RPPAL.
    InvalidChannel,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Error::Io(ref err) => write!(f, "I/O error: {}", err),
            Error::UnknownModel => write!(f, "Unknown Raspberry Pi model"),
            Error::InvalidChannel => write!(f, "Invalid PWM channel"),
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
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Channel {
    Pwm0 = 0,
    Pwm1 = 1,
    Pwm2 = 2,
    Pwm3 = 3,
}

impl fmt::Display for Channel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Channel::Pwm0 => write!(f, "Pwm0"),
            Channel::Pwm1 => write!(f, "Pwm1"),
            Channel::Pwm2 => write!(f, "Pwm2"),
            Channel::Pwm3 => write!(f, "Pwm3"),
        }
    }
}

impl TryFrom<u8> for Channel {
    type Error = Error;

    fn try_from(value: u8) -> result::Result<Self, Self::Error> {
        match value {
            0 => Ok(Channel::Pwm0),
            1 => Ok(Channel::Pwm1),
            2 => Ok(Channel::Pwm2),
            3 => Ok(Channel::Pwm3),
            _ => Err(Error::InvalidChannel),
        }
    }
}

/// Output polarities.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Polarity {
    Normal,
    Inverse,
}

impl fmt::Display for Polarity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Polarity::Normal => write!(f, "Normal"),
            Polarity::Inverse => write!(f, "Inverse"),
        }
    }
}

/// Provides access to the Raspberry Pi's PWM peripheral.
///
/// Before using `Pwm`, make sure the selected PWM channel has been configured
/// and activated. More information can be found [here].
///
/// The `embedded-hal` trait implementations for `Pwm` can be enabled by specifying
/// the optional `hal` feature in the dependency declaration for the `rppal` crate.
///
/// [here]: index.html
#[derive(Debug)]
pub struct Pwm {
    chip: u8,
    channel: u8,
    reset_on_drop: bool,
}

impl Pwm {
    /// Constructs a new `Pwm`.
    ///
    /// `new` attempts to select the correct pwmchip device and channel index based
    /// on the Raspberry Pi model. Use `with_pwmchip` for non-standard configurations.
    ///
    /// `new` doesn't change the channel's period, pulse width or polarity. The channel
    /// will remain disabled until [`enable`] is called.
    ///
    /// [`enable`]: #method.enable
    pub fn new(channel: Channel) -> Result<Pwm> {
        // Select chip/channel based on Pi model
        let device_info = DeviceInfo::new().map_err(|_| Error::UnknownModel)?;

        let pwmchip = device_info.pwm_chip();
        let index = channel as u8;

        Self::with_pwmchip(pwmchip, index)
    }

    /// Constructs a new `Pwm` using the specified pwmchip and channel index.
    ///
    /// Use this method to address PWM channels with non-standard configurations on
    /// different pwmchip devices, or that fall outside the standard 4 PWM channel range.
    ///
    /// `with_pwmchip` doesn't change the channel's period, pulse width or polarity. The channel
    /// will remain disabled until [`enable`] is called.
    ///
    /// [`enable`]: #method.enable
    pub fn with_pwmchip(pwmchip: u8, index: u8) -> Result<Pwm> {
        sysfs::export(pwmchip, index)?;

        let pwm = Pwm {
            chip: pwmchip,
            channel: index,
            reset_on_drop: true,
        };

        // Always reset "enable" to 0. The sysfs interface has a bug where a previous
        // export may have left "enable" as 1 after unexporting. On the next export,
        // "enable" is still set to 1, even though the channel isn't enabled.
        let _ = pwm.disable();

        Ok(pwm)
    }

    /// Constructs a new `Pwm` using the specified settings.
    ///
    /// `period` indicates the time it takes for the PWM channel to complete one cycle.
    ///
    /// `pulse_width` indicates the amount of time the PWM channel is active during a
    /// single period.
    ///
    /// `polarity` configures the active logic level as either high ([`Normal`])
    /// or low ([`Inverse`]).
    ///
    /// `enabled` enables PWM on the selected channel. If `enabled` is set to `false`,
    /// the channel will remain disabled until [`enable`] is called.
    ///
    /// This method will fail if `period` is shorter than `pulse_width`.
    ///
    /// [`Normal`]: enum.Polarity.html#variant.Normal
    /// [`Inverse`]: enum.Polarity.html#variant.Inverse
    /// [`enable`]: #method.enable
    pub fn with_period(
        channel: Channel,
        period: Duration,
        pulse_width: Duration,
        polarity: Polarity,
        enabled: bool,
    ) -> Result<Pwm> {
        let pwm = Pwm::new(channel)?;

        // Set pulse width to 0 first in case the new period is shorter than the current pulse width
        let _ = sysfs::set_pulse_width(pwm.chip, pwm.channel, 0);

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
    /// `frequency` is specified in hertz (Hz).
    ///
    /// `duty_cycle` is specified as a floating point value between `0.0` (0%) and `1.0` (100%).
    ///
    /// `polarity` configures the active logic level as either high ([`Normal`])
    /// or low ([`Inverse`]).
    ///
    /// `enabled` enables PWM on the selected channel. If `enabled` is set to `false`,
    /// the channel will remain disabled until [`enable`] is called.
    ///
    /// [`Normal`]: enum.Polarity.html#variant.Normal
    /// [`Inverse`]: enum.Polarity.html#variant.Inverse
    /// [`enable`]: #method.enable
    pub fn with_frequency(
        channel: Channel,
        frequency: f64,
        duty_cycle: f64,
        polarity: Polarity,
        enabled: bool,
    ) -> Result<Pwm> {
        let pwm = Pwm::new(channel)?;

        // Set pulse width to 0 first in case the new period is shorter than the current pulse width
        let _ = sysfs::set_pulse_width(pwm.chip, pwm.channel, 0);

        // Convert to nanoseconds
        let period = if frequency == 0.0 {
            0.0
        } else {
            (1.0 / frequency) * NANOS_PER_SEC
        };
        let pulse_width = period * duty_cycle.clamp(0.0, 1.0);

        sysfs::set_period(pwm.chip, pwm.channel, period as u64)?;
        sysfs::set_pulse_width(pwm.chip, pwm.channel, pulse_width as u64)?;
        pwm.set_polarity(polarity)?;
        if enabled {
            pwm.enable()?;
        }

        Ok(pwm)
    }

    /// Returns the period.
    pub fn period(&self) -> Result<Duration> {
        Ok(Duration::from_nanos(sysfs::period(
            self.chip,
            self.channel,
        )?))
    }

    /// Sets the period.
    ///
    /// `period` indicates the time it takes for the PWM channel to complete one cycle.
    ///
    /// This method will fail if `period` is shorter than the current pulse width.
    pub fn set_period(&self, period: Duration) -> Result<()> {
        sysfs::set_period(
            self.chip,
            self.channel,
            u64::from(period.subsec_nanos())
                .saturating_add(period.as_secs().saturating_mul(NANOS_PER_SEC as u64)),
        )?;

        Ok(())
    }

    /// Returns the pulse width.
    pub fn pulse_width(&self) -> Result<Duration> {
        Ok(Duration::from_nanos(sysfs::pulse_width(
            self.chip,
            self.channel,
        )?))
    }

    /// Sets the pulse width.
    ///
    /// `pulse_width` indicates the amount of time the PWM channel is active during a
    /// single period.
    ///
    /// This method will fail if `pulse_width` is longer than the current period.
    pub fn set_pulse_width(&self, pulse_width: Duration) -> Result<()> {
        sysfs::set_pulse_width(
            self.chip,
            self.channel,
            u64::from(pulse_width.subsec_nanos())
                .saturating_add(pulse_width.as_secs().saturating_mul(NANOS_PER_SEC as u64)),
        )?;

        Ok(())
    }

    /// Returns the frequency.
    ///
    /// `frequency` is a convenience method that calculates the frequency in hertz (Hz)
    /// based on the configured period.
    pub fn frequency(&self) -> Result<f64> {
        let period = sysfs::period(self.chip, self.channel)? as f64;

        Ok(if period == 0.0 {
            0.0
        } else {
            1.0 / (period / NANOS_PER_SEC)
        })
    }

    /// Sets the frequency and duty cycle.
    ///
    /// `set_frequency` is a convenience method that converts `frequency` to a period,
    /// and calculates the duty cycle as a percentage of the frequency.
    ///
    /// `frequency` is specified in hertz (Hz).
    ///
    /// `duty_cycle` is specified as a floating point value between `0.0` (0%) and `1.0` (100%).
    pub fn set_frequency(&self, frequency: f64, duty_cycle: f64) -> Result<()> {
        // Set duty cycle to 0 first in case the new period is shorter than the current duty cycle
        let _ = sysfs::set_pulse_width(self.chip, self.channel, 0);

        // Convert to nanoseconds
        let period = if frequency == 0.0 {
            0.0
        } else {
            (1.0 / frequency) * NANOS_PER_SEC
        };
        let pulse_width = period * duty_cycle.clamp(0.0, 1.0);

        sysfs::set_period(self.chip, self.channel, period as u64)?;
        sysfs::set_pulse_width(self.chip, self.channel, pulse_width as u64)?;

        Ok(())
    }

    /// Returns the duty cycle.
    ///
    /// `duty_cycle` is a convenience method that calculates the duty cycle as a
    /// floating point value between `0.0` (0%) and `1.0` (100%) based on the configured
    /// period and pulse width.
    pub fn duty_cycle(&self) -> Result<f64> {
        let period = sysfs::period(self.chip, self.channel)? as f64;
        let pulse_width = sysfs::pulse_width(self.chip, self.channel)? as f64;

        Ok(if period == 0.0 {
            0.0
        } else {
            (pulse_width / period).clamp(0.0, 1.0)
        })
    }

    /// Sets the duty cycle.
    ///
    /// `set_duty_cycle` is a convenience method that converts `duty_cycle` to a
    /// pulse width based on the configured period.
    ///
    /// `duty_cycle` is specified as a floating point value between `0.0` (0%) and `1.0` (100%).
    pub fn set_duty_cycle(&self, duty_cycle: f64) -> Result<()> {
        let period = sysfs::period(self.chip, self.channel)? as f64;
        let pulse_width = period * duty_cycle.clamp(0.0, 1.0);

        sysfs::set_pulse_width(self.chip, self.channel, pulse_width as u64)?;

        Ok(())
    }

    /// Returns the polarity.
    pub fn polarity(&self) -> Result<Polarity> {
        Ok(sysfs::polarity(self.chip, self.channel)?)
    }

    /// Sets the polarity.
    ///
    /// `polarity` configures the active logic level as either high
    /// ([`Normal`]) or low ([`Inverse`]).
    ///
    /// [`Normal`]: enum.Polarity.html#variant.Normal
    /// [`Inverse`]: enum.Polarity.html#variant.Inverse
    pub fn set_polarity(&self, polarity: Polarity) -> Result<()> {
        sysfs::set_polarity(self.chip, self.channel, polarity)?;

        Ok(())
    }

    /// Returns `true` if the PWM channel is enabled.
    pub fn is_enabled(&self) -> Result<bool> {
        Ok(sysfs::enabled(self.chip, self.channel)?)
    }

    /// Enables the PWM channel.
    pub fn enable(&self) -> Result<()> {
        sysfs::set_enabled(self.chip, self.channel, true)?;

        Ok(())
    }

    /// Disables the PWM channel.
    pub fn disable(&self) -> Result<()> {
        sysfs::set_enabled(self.chip, self.channel, false)?;

        Ok(())
    }

    /// Returns the value of `reset_on_drop`.
    pub fn reset_on_drop(&self) -> bool {
        self.reset_on_drop
    }

    /// When enabled, disables the PWM channel when the `Pwm` instance
    /// goes out of scope. By default, this is set to `true`.
    ///
    /// ## Note
    ///
    /// Drop methods aren't called when a process is abnormally terminated, for
    /// instance when a user presses <kbd>Ctrl</kbd> + <kbd>C</kbd>, and the `SIGINT` signal
    /// isn't caught. You can catch those using crates such as [`simple_signal`].
    ///
    /// [`simple_signal`]: https://crates.io/crates/simple-signal
    pub fn set_reset_on_drop(&mut self, reset_on_drop: bool) {
        self.reset_on_drop = reset_on_drop;
    }
}

impl Drop for Pwm {
    fn drop(&mut self) {
        if self.reset_on_drop {
            let _ = sysfs::set_enabled(self.chip, self.channel, false);
            let _ = sysfs::unexport(self.chip, self.channel);
        }
    }
}

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

//! Interface for the GPIO peripheral.
//!
//! To ensure fast performance, RPPAL interfaces with the GPIO peripheral by
//! directly accessing the registers through either `/dev/gpiomem` or `/dev/mem`.
//! GPIO interrupts are controlled using the `/dev/gpiochipN` (where N=0, 1 and 2)
//! character device.
//!
//! ## Pins
//!
//! Pins are addressed by their BCM numbers, rather than their
//! physical location.
//!
//! By default, pins are reset to their original state when they go out of scope.
//! Use [`InputPin::set_clear_on_drop(false)`], [`OutputPin::set_clear_on_drop(false)`]
//! or [`AltPin::set_clear_on_drop(false)`], respecively, to disable this behavior.
//! Note that `drop` methods aren't called when a program is abnormally terminated (for
//! instance when a SIGINT isn't caught).
//!
//! ## Permission denied
//!
//! In recent releases of Raspbian (December 2017 or later), users that are part of the
//! `gpio` group (like the default `pi` user) can access `/dev/gpiomem` and
//! `/dev/gpiochipN` without needing additional permissions. If you encounter any
//! Permission Denied errors when creating a new [`Gpio`] instance, either the current
//! user isn't a member of the `gpio` group, or your Raspbian distribution isn't
//! up-to-date and doesn't automatically configure permissions for the above-mentioned
//! files. Updating Raspbian to the latest release should fix any permission issues.
//! Alternatively, although not recommended, you can run your application with superuser
//! privileges by using `sudo`.
//!
//! If you're unable to update Raspbian and its packages (namely `raspberrypi-sys-mods`) to
//! the latest available release, or updating hasn't fixed the issue, you might be able to
//! manually update your udev rules to set the appropriate permissions. More information
//! can be found at [raspberrypi/linux#1225] and [raspberrypi/linux#2289].
//!
//! [raspberrypi/linux#1225]: https://github.com/raspberrypi/linux/issues/1225
//! [raspberrypi/linux#2289]: https://github.com/raspberrypi/linux/issues/2289
//! [`Gpio`]: struct.Gpio.html
//! [`InputPin::set_clear_on_drop(false)`]: struct.InputPin.html#method.set_clear_on_drop
//! [`OutputPin::set_clear_on_drop(false)`]: struct.InputPin.html#method.set_clear_on_drop
//! [`AltPin::set_clear_on_drop(false)`]: struct.InputPin.html#method.set_clear_on_drop
//! [`Error::InstanceExists`]: enum.Error.html#variant.InstanceExists
//!
//! ## Examples
//!
//! Basic example:
//!
//! ```
//! use std::thread::sleep;
//! use std::time::Duration;
//!
//! use rppal::gpio::Gpio;
//!
//! # fn main() -> rppal::gpio::Result<()> {
//! let gpio = Gpio::new()?;
//! let mut pin = gpio.get(23).unwrap().into_output();
//!
//! pin.set_high();
//! sleep(Duration::from_secs(1));
//! pin.set_low();
//! # Ok(())
//! # }
//! ```

use std::fmt;
use std::io;
use std::os::unix::io::AsRawFd;
use std::result;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;

use lazy_static::lazy_static;
use quick_error::quick_error;

mod epoll;
mod interrupt;
mod ioctl;
mod mem;
mod pin;

pub use self::pin::{AltPin, InputPin, OutputPin, Pin};

quick_error! {
/// Errors that can occur when accessing the GPIO peripheral.
    #[derive(Debug)]
    pub enum Error {
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
        UnknownModel { description("unknown Raspberry Pi model") }
/// Permission denied when opening `/dev/gpiomem`, `/dev/mem` or `/dev/gpiochipN` for
/// read/write access.
///
/// More information on possible causes for this error can be found [here].
///
/// [here]: index.html##permission-denied
        PermissionDenied { description("/dev/gpiomem and/or /dev/mem insufficient permissions") }
/// IO error.
        Io(err: io::Error) { description(err.description()) from() }
/// Interrupt polling thread panicked.
        ThreadPanic { description("interrupt polling thread panicked") }
    }
}

/// Result type returned from methods that can have `rppal::gpio::Error`s.
pub type Result<T> = result::Result<T, Error>;

/// Pin modes.
#[derive(Debug, PartialEq, Copy, Clone)]
#[repr(u8)]
pub enum Mode {
    Input = 0b000,
    Output = 0b001,
    Alt5 = 0b010, // PWM
    Alt4 = 0b011, // SPI
    Alt0 = 0b100, // PCM
    Alt1 = 0b101, // SMI
    Alt2 = 0b110, // ---
    Alt3 = 0b111, // BSC-SPI
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Mode::Input => write!(f, "In"),
            Mode::Output => write!(f, "Out"),
            Mode::Alt0 => write!(f, "Alt0"),
            Mode::Alt1 => write!(f, "Alt1"),
            Mode::Alt2 => write!(f, "Alt2"),
            Mode::Alt3 => write!(f, "Alt3"),
            Mode::Alt4 => write!(f, "Alt4"),
            Mode::Alt5 => write!(f, "Alt5"),
        }
    }
}

/// Pin logic levels.
#[derive(Debug, PartialEq, Copy, Clone)]
#[repr(u8)]
pub enum Level {
    Low = 0,
    High = 1,
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Level::Low => write!(f, "Low"),
            Level::High => write!(f, "High"),
        }
    }
}

/// Built-in pull-up/pull-down resistor states.
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum PullUpDown {
    Off = 0b00,
    PullDown = 0b01,
    PullUp = 0b10,
}

impl fmt::Display for PullUpDown {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            PullUpDown::Off => write!(f, "Off"),
            PullUpDown::PullDown => write!(f, "PullDown"),
            PullUpDown::PullUp => write!(f, "PullUp"),
        }
    }
}

/// Interrupt trigger conditions.
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Trigger {
    Disabled = 0,
    RisingEdge = 1,
    FallingEdge = 2,
    Both = 3,
}

impl fmt::Display for Trigger {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Trigger::Disabled => write!(f, "Disabled"),
            Trigger::RisingEdge => write!(f, "RisingEdge"),
            Trigger::FallingEdge => write!(f, "FallingEdge"),
            Trigger::Both => write!(f, "Both"),
        }
    }
}

// Store Gpio's state separately, so we can conveniently share it through
// a cloned Arc.
pub(crate) struct GpioState {
    gpio_mem: mem::GpioMem,
    cdev: std::fs::File,
    sync_interrupts: Mutex<interrupt::EventLoop>,
    pins_taken: [AtomicBool; pin::MAX],
}

impl fmt::Debug for GpioState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventLoop")
            .field("gpio_mem", &self.gpio_mem)
            .field("cdev", &self.cdev)
            .field("sync_interrupts", &self.sync_interrupts)
            .field("pins_taken", &format_args!("{{ .. }}"))
            .finish()
    }
}

// Share state between Gpio and Pin instances. GpioState is dropped after
// all Gpio and Pin instances go out of scope, guaranteeing we won't have
// any pins simultaneously using different EventLoop or GpioMem instances.
lazy_static! {
    static ref GPIO_STATE: Mutex<Weak<GpioState>> = Mutex::new(Weak::new());
}

/// Provides access to the Raspberry Pi's GPIO peripheral.
#[derive(Clone, Debug)]
pub struct Gpio {
    inner: Arc<GpioState>,
}

impl Gpio {
    /// Constructs a new `Gpio`.
    pub fn new() -> Result<Gpio> {
        let mut static_state = GPIO_STATE.lock().unwrap();

        // Create a strong reference if a GpioState instance already exists,
        // otherwise initialize it here so we can return any relevant errors.
        if let Some(ref state) = static_state.upgrade() {
            Ok(Gpio {
                inner: state.clone(),
            })
        } else {
            let gpio_mem = mem::GpioMem::open()?;
            let cdev = ioctl::find_gpiochip()?;
            let sync_interrupts =
                Mutex::new(interrupt::EventLoop::new(cdev.as_raw_fd(), pin::MAX)?);
            let pins_taken = init_array!(AtomicBool::new(false), pin::MAX);

            let gpio_state = Arc::new(GpioState {
                gpio_mem,
                cdev,
                sync_interrupts,
                pins_taken,
            });

            // Store a weak reference to our state. This gets dropped when
            // all Gpio and Pin instances go out of scope.
            *static_state = Arc::downgrade(&gpio_state);

            Ok(Gpio { inner: gpio_state })
        }
    }

    /// Returns a [`Pin`] for the specified GPIO pin number.
    ///
    /// Retrieving a GPIO pin using `get` grants exclusive access to the GPIO
    /// pin through an owned [`Pin`]. If the selected pin number is already
    /// in use, `get` returns `None`. After a [`Pin`] goes out of scope, it can be retrieved
    /// again using `get`.
    ///
    /// [`Pin`]: struct.Pin.html
    pub fn get(&self, pin: u8) -> Option<pin::Pin> {
        if pin as usize >= pin::MAX {
            return None;
        }

        // Returns true if the pin is currently taken, otherwise atomically sets
        // it to true here
        if self.inner.pins_taken[pin as usize].compare_and_swap(false, true, Ordering::SeqCst) {
            // Pin is currently taken
            None
        } else {
            // Return an owned Pin
            let pin_instance = pin::Pin::new(pin, self.inner.clone());

            Some(pin_instance)
        }
    }

    /// Blocks until an interrupt is triggered on any of the specified pins, or until a timeout occurs.
    ///
    /// This only works for pins that have been configured for synchronous interrupts using
    /// [`InputPin::set_interrupt`]. Asynchronous interrupt triggers are automatically polled on a separate thread.
    ///
    /// If `reset` is set to `false`, returns immediately if an interrupt trigger event was cached in a
    /// previous call to [`InputPin::poll_interrupt`] or `poll_interrupts`.
    /// If `reset` is set to `true`, clears any cached interrupt trigger events before polling.
    ///
    /// The `timeout` duration indicates how long the call to `poll_interrupts` will block while waiting
    /// for interrupt trigger events, after which an `Ok(None))` is returned.
    /// `timeout` can be set to `None` to wait indefinitely.
    ///
    /// When an interrupt event is triggered, `poll_interrupts` returns
    /// `Ok((&`[`InputPin`]`, `[`Level`]`))` containing the corresponding pin and logic level. If multiple events trigger
    /// at the same time, only the first one is returned. The remaining events are cached and will be returned
    /// the next time [`InputPin::poll_interrupt`] or `poll_interrupts` is called.
    ///
    /// [`InputPin::set_interrupt`]: struct.InputPin#method.set_interrupt
    /// [`InputPin::poll_interrupt`]: struct.InputPin#method.poll_interrupt
    /// [`InputPin`]: struct.InputPin
    /// [`Level`]: struct.Level
    pub fn poll_interrupts<'a>(
        &self,
        pins: &[&'a InputPin],
        reset: bool,
        timeout: Option<Duration>,
    ) -> Result<Option<(&'a InputPin, Level)>> {
        (*self.inner.sync_interrupts.lock().unwrap()).poll(pins, reset, timeout)
    }
}

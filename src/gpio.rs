//! Interface for the GPIO peripheral.
//!
//! To ensure fast performance, RPPAL controls the GPIO peripheral by directly
//! accessing the registers through either `/dev/gpiomem` or `/dev/mem`. GPIO interrupts
//! are configured using the `gpiochip` character device.
//!
//! ## Pins
//!
//! GPIO pins are retrieved from a [`Gpio`] instance by their BCM GPIO number by calling
//! [`Gpio::get`]. The returned unconfigured [`Pin`] can be used to read the pin's
//! mode and logic level. Converting the [`Pin`] to an [`InputPin`], [`OutputPin`] or
//! [`IoPin`] through the various `into_` methods available on [`Pin`] configures the
//! appropriate mode, and provides access to additional methods relevant to the selected pin mode.
//!
//! Retrieving a GPIO pin with [`Gpio::get`] grants access to the pin through an owned [`Pin`]
//! instance. If the pin is already in use, or the GPIO peripheral doesn't expose a pin with
//! the specified number, [`Gpio::get`] returns `Err(`[`Error::PinNotAvailable`]`)`. After a [`Pin`]
//! (or a derived [`InputPin`], [`OutputPin`] or [`IoPin`]) goes out of scope, it can be
//! retrieved again through another [`Gpio::get`] call.
//!
//! By default, pins are reset to their original state when they go out of scope.
//! Use [`InputPin::set_reset_on_drop(false)`], [`OutputPin::set_reset_on_drop(false)`]
//! or [`IoPin::set_reset_on_drop(false)`], respectively, to disable this behavior.
//! Note that `drop` methods aren't called when a process is abnormally terminated (for
//! instance when a `SIGINT` signal isn't caught).
//!
//! ## Interrupts
//!
//! [`InputPin`] supports both synchronous and asynchronous interrupt handlers.
//!
//! Synchronous (blocking) interrupt triggers are configured using [`InputPin::set_interrupt`].
//! An interrupt trigger for a single pin can be polled with [`InputPin::poll_interrupt`],
//! which blocks the current thread until a trigger event occurs, or until the timeout period
//! elapses. [`Gpio::poll_interrupts`] should be used when multiple pins have been configured
//! for synchronous interrupt triggers, and need to be polled simultaneously.
//!
//! Asynchronous interrupt triggers are configured using [`InputPin::set_async_interrupt`]. The
//! specified callback function will be executed on a separate thread when a trigger event occurs.
//!
//! ## Software-based PWM
//!
//! [`OutputPin`] and [`IoPin`] feature a software-based PWM implementation. The PWM signal is
//! emulated by toggling the pin's output state on a separate thread, combined with sleep and
//! busy-waiting.
//!
//! Software-based PWM is inherently inaccurate on a multi-threaded OS due to scheduling/preemption.
//! If an accurate or faster PWM signal is required, use the hardware [`Pwm`] peripheral instead.
//!
//! PWM threads may occasionally sleep longer than needed. If the active or inactive part of the
//! signal is shorter than 250 µs, only busy-waiting is used, which will increase CPU usage. Due to
//! function call overhead, typical jitter is expected to be up to 10 µs on debug builds, and up to
//! 2 µs on release builds.
//!
//! ## Examples
//!
//! Basic example:
//!
//! ```
//! use std::thread;
//! use std::time::Duration;
//!
//! use rppal::gpio::Gpio;
//!
//! # fn main() -> rppal::gpio::Result<()> {
//! let gpio = Gpio::new()?;
//! let mut pin = gpio.get(23)?.into_output();
//!
//! pin.set_high();
//! thread::sleep(Duration::from_secs(1));
//! pin.set_low();
//! # Ok(())
//! # }
//! ```
//!
//! Additional examples can be found in the `examples` directory.
//!
//! ## Troubleshooting
//!
//! ### Permission denied
//!
//! In recent releases of Raspberry Pi OS (December 2017 or later), users that are part of the
//! `gpio` group (like the default `pi` user) can access `/dev/gpiomem` and
//! `/dev/gpiochipN` (N = 0-2) without needing additional permissions. If you encounter any
//! [`PermissionDenied`] errors when constructing a new [`Gpio`] instance, either the current
//! user isn't a member of the `gpio` group, or your Raspberry Pi OS distribution isn't
//! up-to-date and doesn't automatically configure permissions for the above-mentioned
//! files. Updating Raspberry Pi OS to the latest release should fix any permission issues.
//! Alternatively, although not recommended, you can run your application with superuser
//! privileges by using `sudo`.
//!
//! If you're unable to update Raspberry Pi OS and its packages (namely `raspberrypi-sys-mods`) to
//! the latest available release, or updating hasn't fixed the issue, you might be able to
//! manually update your `udev` rules to set the appropriate permissions. More information
//! can be found at [raspberrypi/linux#1225] and [raspberrypi/linux#2289].
//!
//! [`Error::PinNotAvailable`]: enum.Error.html#variant.PinNotAvailable
//! [`PermissionDenied`]: enum.Error.html#variant.PermissionDenied
//! [raspberrypi/linux#1225]: https://github.com/raspberrypi/linux/issues/1225
//! [raspberrypi/linux#2289]: https://github.com/raspberrypi/linux/issues/2289
//! [`Gpio`]: struct.Gpio.html
//! [`Gpio::get`]: struct.Gpio.html#method.get
//! [`Gpio::poll_interrupts`]: struct.Gpio.html#method.poll_interrupts
//! [`Pin`]: struct.Pin.html
//! [`InputPin`]: struct.InputPin.html
//! [`InputPin::set_reset_on_drop(false)`]: struct.InputPin.html#method.set_reset_on_drop
//! [`InputPin::set_interrupt`]: struct.InputPin.html#method.set_interrupt
//! [`InputPin::poll_interrupt`]: struct.InputPin.html#method.poll_interrupt
//! [`InputPin::set_async_interrupt`]: struct.InputPin.html#method.set_async_interrupt
//! [`OutputPin`]: struct.OutputPin.html
//! [`OutputPin::set_reset_on_drop(false)`]: struct.OutputPin.html#method.set_reset_on_drop
//! [`IoPin`]: struct.IoPin.html
//! [`IoPin::set_reset_on_drop(false)`]: struct.IoPin.html#method.set_reset_on_drop
//! [`Pwm`]: ../pwm/struct.Pwm.html

use std::error;
use std::fmt;
use std::io;
use std::mem::MaybeUninit;
use std::ops::Not;
use std::os::unix::io::AsRawFd;
use std::result;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, Once, Weak};
use std::time::Duration;

mod epoll;
mod gpiomem;
#[cfg(feature = "hal")]
mod hal;
#[cfg(feature = "hal-unproven")]
mod hal_unproven;
mod interrupt;
mod ioctl;
mod pin;
mod soft_pwm;

use crate::system;
use crate::system::DeviceInfo;

pub use self::pin::{InputPin, IoPin, OutputPin, Pin};

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
    UnknownModel,
    /// Pin is already in use.
    ///
    /// The pin is already in use elsewhere in your application. If the pin is currently in
    /// use, you may retrieve it again after the [`Pin`] (or a derived [`InputPin`],
    /// [`OutputPin`] or [`IoPin`]) instance goes out of scope.
    ///
    /// [`Pin`]: struct.Pin.html
    /// [`InputPin`]: struct.InputPin.html
    /// [`OutputPin`]: struct.OutputPin.html
    /// [`IoPin`]: struct.IoPin.html
    PinUsed(u8),
    /// Pin is not available.
    ///
    /// The GPIO peripheral doesn't expose a GPIO pin with the specified number. Pins are
    /// addressed by their BCM GPIO numbers, rather than their physical location on the GPIO
    /// header.
    PinNotAvailable(u8),
    /// Permission denied when opening `/dev/gpiomem`, `/dev/mem` or `/dev/gpiochipN` for
    /// read/write access.
    ///
    /// More information on possible causes for this error can be found [here].
    ///
    /// [here]: index.html#permission-denied
    PermissionDenied(String),
    /// I/O error.
    Io(io::Error),
    /// Thread panicked.
    ThreadPanic,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Error::UnknownModel => write!(f, "Unknown Raspberry Pi model"),
            Error::PinUsed(pin) => write!(f, "Pin {} is already in use", pin),
            Error::PinNotAvailable(pin) => write!(f, "Pin {} is not available", pin),
            Error::PermissionDenied(ref path) => write!(f, "Permission denied: {}", path),
            Error::Io(ref err) => write!(f, "I/O error: {}", err),
            Error::ThreadPanic => write!(f, "Thread panicked"),
        }
    }
}

impl error::Error for Error {}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}

impl From<system::Error> for Error {
    fn from(_err: system::Error) -> Error {
        Error::UnknownModel
    }
}

/// Result type returned from methods that can have `rppal::gpio::Error`s.
pub type Result<T> = result::Result<T, Error>;

/// Pin modes.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
#[repr(u8)]
pub enum Mode {
    Input,
    Output,
    Alt0,
    Alt1,
    Alt2,
    Alt3,
    Alt4,
    Alt5,
    Alt6,
    Alt7,
    Alt8,
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
            Mode::Alt6 => write!(f, "Alt6"),
            Mode::Alt7 => write!(f, "Alt7"),
            Mode::Alt8 => write!(f, "Alt8"),
        }
    }
}

/// Pin logic levels.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
#[repr(u8)]
pub enum Level {
    Low = 0,
    High = 1,
}

impl From<bool> for Level {
    fn from(e: bool) -> Level {
        if e {
            Level::High
        } else {
            Level::Low
        }
    }
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Level::Low => write!(f, "Low"),
            Level::High => write!(f, "High"),
        }
    }
}

impl From<u8> for Level {
    fn from(value: u8) -> Self {
        if value == 0 {
            Level::Low
        } else {
            Level::High
        }
    }
}

impl Not for Level {
    type Output = Level;

    fn not(self) -> Level {
        match self {
            Level::Low => Level::High,
            Level::High => Level::Low,
        }
    }
}

/// Built-in pull-up/pull-down resistor states.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
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
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
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
    gpio_mem: Box<dyn gpiomem::GpioRegisters>,
    cdev: std::fs::File,
    sync_interrupts: Mutex<interrupt::EventLoop>,
    pins_taken: [AtomicBool; u8::MAX as usize],
    gpio_lines: u8,
}

impl fmt::Debug for GpioState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventLoop")
            .field("gpio_mem", &self.gpio_mem)
            .field("cdev", &self.cdev)
            .field("sync_interrupts", &self.sync_interrupts)
            .field("pins_taken", &format_args!("{{ .. }}"))
            .field("gpio_lines", &self.gpio_lines)
            .finish()
    }
}

/// Provides access to the Raspberry Pi's GPIO peripheral.
#[derive(Clone, Debug)]
pub struct Gpio {
    inner: Arc<GpioState>,
}

impl Gpio {
    /// Constructs a new `Gpio`.
    pub fn new() -> Result<Gpio> {
        // Replace this when std::sync::SyncLazy is stabilized. https://github.com/rust-lang/rust/issues/74465

        // Shared state between Gpio and Pin instances. GpioState is dropped after
        // all Gpio and Pin instances go out of scope, guaranteeing we won't have
        // any pins simultaneously using different EventLoop or GpioMem instances.
        static mut GPIO_STATE: MaybeUninit<Mutex<Weak<GpioState>>> = MaybeUninit::uninit();
        static ONCE: Once = Once::new();

        // call_once is thread-safe, guaranteed to be called only once, and memory writes performed
        // by the closure can be observed by other threads after execution completes.
        let mut weak_state = unsafe {
            ONCE.call_once(|| {
                GPIO_STATE.write(Mutex::new(Weak::new()));
            });

            // GPIO_STATE will always be initialized at this point.
            GPIO_STATE.assume_init_ref().lock().unwrap()
        };

        // Clone a strong reference if a GpioState instance already exists, otherwise
        // initialize it here so we can return any relevant errors.
        if let Some(ref state) = weak_state.upgrade() {
            Ok(Gpio {
                inner: state.clone(),
            })
        } else {
            let device_info = DeviceInfo::new().map_err(|_| Error::UnknownModel)?;

            let gpio_mem: Box<dyn gpiomem::GpioRegisters> = match device_info.gpio_interface() {
                system::GpioInterface::Bcm => Box::new(gpiomem::bcm::GpioMem::open()?),
                system::GpioInterface::Rp1 => Box::new(gpiomem::rp1::GpioMem::open()?),
            };

            let cdev = ioctl::find_gpiochip()?;
            let sync_interrupts = Mutex::new(interrupt::EventLoop::new(
                cdev.as_raw_fd(),
                u8::MAX as usize,
            )?);
            let pins_taken = init_array!(AtomicBool::new(false), u8::MAX as usize);
            let gpio_lines = device_info.gpio_lines();

            let gpio_state = Arc::new(GpioState {
                gpio_mem,
                cdev,
                sync_interrupts,
                pins_taken,
                gpio_lines,
            });

            // Store a weak reference to our state. This gets dropped when
            // all Gpio and Pin instances go out of scope.
            *weak_state = Arc::downgrade(&gpio_state);

            Ok(Gpio { inner: gpio_state })
        }
    }

    /// Returns a [`Pin`] for the specified BCM GPIO number.
    ///
    /// Retrieving a GPIO pin grants access to the pin through an owned [`Pin`] instance.
    /// If the pin is already in use, `get` returns `Err(`[`Error::PinUsed`]`)`.
    /// After a [`Pin`] (or a derived [`InputPin`], [`OutputPin`] or [`IoPin`]) goes out
    /// of scope, it can be retrieved again through another `get` call.
    ///
    /// [`Pin`]: struct.Pin.html
    /// [`InputPin`]: struct.InputPin.html
    /// [`OutputPin`]: struct.OutputPin.html
    /// [`IoPin`]: struct.IoPin.html
    /// [`Error::PinUsed`]: enum.Error.html#variant.PinUsed
    pub fn get(&self, pin: u8) -> Result<Pin> {
        if pin >= self.inner.gpio_lines {
            return Err(Error::PinNotAvailable(pin));
        }

        // Returns an error if the pin is already taken, otherwise atomically sets it to true here
        if self.inner.pins_taken[pin as usize]
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            // Pin is taken
            Err(Error::PinUsed(pin))
        } else {
            // Return an owned Pin
            Ok(Pin::new(pin, self.inner.clone()))
        }
    }

    /// Blocks until an interrupt is triggered on any of the specified pins, or until a timeout occurs.
    ///
    /// Only pins that have been previously configured for synchronous interrupts using [`InputPin::set_interrupt`]
    /// can be polled. Asynchronous interrupt triggers are automatically polled on a separate thread.
    ///
    /// Calling `poll_interrupts` blocks any other calls to `poll_interrupts` or [`InputPin::poll_interrupt`] until
    /// it returns. If you need to poll multiple pins simultaneously on different threads, consider using
    /// asynchronous interrupts with [`InputPin::set_async_interrupt`] instead.
    ///
    /// Setting `reset` to `false` returns any cached interrupt trigger events if available. Setting `reset` to `true`
    /// clears all cached events before polling for new events.
    ///
    /// The `timeout` duration indicates how long the call to `poll_interrupts` will block while waiting
    /// for interrupt trigger events, after which an `Ok(None)` is returned.
    /// `timeout` can be set to `None` to wait indefinitely.
    ///
    /// When an interrupt event is triggered, `poll_interrupts` returns
    /// `Ok((&`[`InputPin`]`, `[`Level`]`))` containing the corresponding pin and logic level. If multiple events trigger
    /// at the same time, only the first one is returned. The remaining events are cached and will be returned
    /// the next time [`InputPin::poll_interrupt`] or `poll_interrupts` is called.
    ///
    /// [`InputPin::set_interrupt`]: struct.InputPin.html#method.set_interrupt
    /// [`InputPin::poll_interrupt`]: struct.InputPin.html#method.poll_interrupt
    /// [`InputPin::set_async_interrupt`]: struct.InputPin.html#method.set_async_interrupt
    /// [`InputPin`]: struct.InputPin.html
    /// [`Level`]: enum.Level.html
    pub fn poll_interrupts<'a>(
        &self,
        pins: &[&'a InputPin],
        reset: bool,
        timeout: Option<Duration>,
    ) -> Result<Option<(&'a InputPin, Level)>> {
        (*self.inner.sync_interrupts.lock().unwrap()).poll(pins, reset, timeout)
    }
}

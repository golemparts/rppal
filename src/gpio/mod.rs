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
//! GPIO interrupts are controlled using the sysfs interface.
//!
//! On a typical up-to-date Raspbian installation, any user that's a member of the
//! `gpio` group can access `/dev/gpiomem`, while `/dev/mem` requires
//! superuser privileges.
//!
//! Pins are addressed by their BCM GPIO pin numbers, rather than their
//! physical location.
//!
//! By default, all pins are reset to their original state when [`Gpio`] goes out
//! of scope. Use [`set_clear_on_drop(false)`] to disable this behavior. Note that
//! drop methods aren't called when a program is abnormally terminated (for
//! instance when a SIGINT isn't caught).
//!
//! Only a single instance of [`Gpio`] can exist at any time. Multiple instances of [`Gpio`]
//! can cause race conditions or pin configuration issues when several threads write to
//! the same register simultaneously. While other applications can't be prevented from
//! writing to the GPIO registers at the same time, limiting [`Gpio`] to a single instance
//! will at least make the Rust interface less error-prone.
//!
//! Constructing another instance before the existing one goes out of scope will return
//! an [`Error::InstanceExists`]. You can share a [`Gpio`] instance with other
//! threads using channels, cloning an `Arc<Mutex<Gpio>>` or globally sharing
//! a `Mutex<Gpio>`.
//!
//! [`Gpio`]: struct.Gpio.html
//! [`set_clear_on_drop(false)`]: struct.Gpio.html#method.set_clear_on_drop
//! [`Error::InstanceExists`]: enum.Error.html#variant.InstanceExists

use std::fmt;
use std::fs::File;
use std::io;
use std::os::unix::io::AsRawFd;
use std::result;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::sleep;
use std::time::Duration;

mod epoll;
mod interrupt;
mod ioctl;
mod mem;

// Maximum GPIO pins on the BCM2835. The actual number of pins exposed through the Pi's GPIO header
// depends on the model.
const GPIO_MAX_PINS: u8 = 54;
// Offset in 32-bit units
const GPIO_OFFSET_GPFSEL: usize = 0;
const GPIO_OFFSET_GPSET: usize = 7;
const GPIO_OFFSET_GPCLR: usize = 10;
const GPIO_OFFSET_GPLEV: usize = 13;
const GPIO_OFFSET_GPPUD: usize = 37;
const GPIO_OFFSET_GPPUDCLK: usize = 38;

// Used to limit Gpio to a single instance
static mut GPIO_INSTANCED: AtomicBool = AtomicBool::new(false);

quick_error! {
/// Errors that can occur when accessing the GPIO peripheral.
    #[derive(Debug)]
    pub enum Error {
/// Invalid GPIO pin number.
///
/// The GPIO pin number is not available on this Raspberry Pi model.
        InvalidPin(pin: u8) { description("invalid GPIO pin number") }
/// Unknown GPIO pin mode.
///
/// The GPIO pin is set to an unknown mode.
        UnknownMode(mode: u8) { description("unknown mode") }
/// Unknown SoC.
///
/// It wasn't possible to automatically identify the Raspberry Pi's SoC.
        UnknownSoC { description("unknown SoC") }
/// Permission denied when opening `/dev/gpiomem` and/or `/dev/mem` for read/write access.
///
/// Make sure the user has read and write access to `/dev/gpiomem`.
/// Common causes are either incorrect file permissions on `/dev/gpiomem`, or
/// the user isn't a member of the `gpio` group. If `/dev/gpiomem` is missing, upgrade to a more
/// recent version of Raspbian.
///
/// `/dev/mem` is a fallback when `/dev/gpiomem` can't be accessed. Getting read and write
/// access to `/dev/mem` is typically accomplished by executing the program as a
/// privileged user through `sudo`. A better solution that doesn't require `sudo` would be
/// to upgrade to a version of Raspbian that implements `/dev/gpiomem`.
        PermissionDenied { description("/dev/gpiomem and/or /dev/mem insufficient permissions") }
/// An instance of [`Gpio`] already exists.
///
/// Multiple instances of [`Gpio`] can cause race conditions or pin configuration issues when
/// several threads write to the same register simultaneously. While other applications
/// can't be prevented from writing to the GPIO registers at the same time, limiting [`Gpio`]
/// to a single instance will at least make the Rust interface less error-prone.
///
/// You can share a [`Gpio`] instance with other threads using channels, or cloning an
/// `Arc<Mutex<Gpio>>`. Although discouraged, you could also share it globally
/// wrapped in a `Mutex` using the `lazy_static` crate.
///
/// [`Gpio`]: struct.Gpio.html
        InstanceExists { description("an instance of Gpio already exists") }
/// [`Gpio`] isn't initialized.
///
/// You should normally only see this error when you call a method after
/// running [`cleanup`].
///
/// [`cleanup`]: struct.Gpio.html#method.cleanup
/// [`Gpio`]: struct.Gpio.html
        NotInitialized { description("not initialized") }
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
pub enum Mode {
    Input = 0b000,
    Output = 0b001,
    Alt0 = 0b100,
    Alt1 = 0b101,
    Alt2 = 0b110,
    Alt3 = 0b111,
    Alt4 = 0b011,
    Alt5 = 0b010,
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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
pub enum Level {
    Low = 0,
    High = 1,
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Trigger::Disabled => write!(f, "Disabled"),
            Trigger::RisingEdge => write!(f, "RisingEdge"),
            Trigger::FallingEdge => write!(f, "FallingEdge"),
            Trigger::Both => write!(f, "Both"),
        }
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
struct PinState {
    pin: u8,
    mode: Mode,
    changed: bool,
}

impl PinState {
    fn new(pin: u8, mode: Mode, changed: bool) -> PinState {
        PinState { pin, mode, changed }
    }
}

/// Provides access to the Raspberry Pi's GPIO peripheral.
pub struct Gpio {
    initialized: bool,
    clear_on_drop: bool,
    gpio_mem: mem::GpioMem,
    orig_pin_state: Vec<PinState>,
    gpio_cdev: File,
    sync_interrupts: interrupt::EventLoop,
    async_interrupts: Vec<Option<interrupt::AsyncInterrupt>>,
}

impl Gpio {
    /// Constructs a new `Gpio`.
    ///
    /// Only a single instance of `Gpio` can exist at any time. Constructing
    /// another instance before the existing one goes out of scope will return
    /// an [`Error::InstanceExists`]. You can share a `Gpio` instance with other
    /// threads using channels, cloning an `Arc<Mutex<Gpio>>` or globally sharing
    /// a `Mutex<Gpio>`.
    ///
    /// [`Error::InstanceExists`]: enum.Error.html#variant.InstanceExists
    pub fn new() -> Result<Gpio> {
        unsafe {
            if GPIO_INSTANCED.load(Ordering::SeqCst) {
                return Err(Error::InstanceExists);
            }
        }

        let cdev = ioctl::find_driver()?;
        let cdev_fd = cdev.as_raw_fd();

        let mut gpio = Gpio {
            initialized: true,
            clear_on_drop: true,
            gpio_mem: mem::GpioMem::new(),
            orig_pin_state: Vec::with_capacity(GPIO_MAX_PINS as usize),
            gpio_cdev: cdev,
            sync_interrupts: interrupt::EventLoop::new(cdev_fd, GPIO_MAX_PINS as usize)?,
            async_interrupts: Vec::with_capacity(GPIO_MAX_PINS as usize),
        };

        gpio.gpio_mem.open()?;

        // Save the original pin states, so we can reset them with cleanup()
        for n in 0..GPIO_MAX_PINS {
            match gpio.mode(n) {
                Ok(mode) => gpio.orig_pin_state.push(PinState::new(n, mode, false)),
                Err(e) => return Err(e),
            }
        }

        // Initialize sync_interrupts while circumventing the Copy/Clone requirement
        for _ in 0..gpio.async_interrupts.capacity() {
            gpio.async_interrupts.push(None);
        }

        unsafe {
            GPIO_INSTANCED.store(true, Ordering::SeqCst);
        }

        Ok(gpio)
    }

    /// Returns the value of `clear_on_drop`.
    pub fn clear_on_drop(&self) -> bool {
        self.clear_on_drop
    }

    /// When enabled, resets all pins to their original state when `Gpio` goes out of scope.
    ///
    /// Drop methods aren't called when a program is abnormally terminated,
    /// for instance when a user presses Ctrl-C, and the SIGINT signal isn't
    /// caught. You'll either have to catch those using crates such as
    /// [`simple_signal`], or manually call [`cleanup`].
    ///
    /// By default, `clear_on_drop` is set to `true`.
    ///
    /// [`simple_signal`]: https://crates.io/crates/simple-signal
    /// [`cleanup`]: #method.cleanup
    pub fn set_clear_on_drop(&mut self, clear_on_drop: bool) {
        self.clear_on_drop = clear_on_drop;
    }

    /// Resets all pins to their original state.
    ///
    /// Normally, this method is automatically called when `Gpio` goes out of
    /// scope, but you can manually call it to handle early/abnormal termination.
    /// After calling this method, any future calls to other methods won't have any
    /// result.
    pub fn cleanup(&mut self) {
        if self.initialized {
            // Use a cloned copy, because set_mode() will try to change
            // the contents of the original vector.
            for pin_state in &self.orig_pin_state.clone() {
                if pin_state.changed {
                    self.set_mode(pin_state.pin, pin_state.mode);
                }
            }

            self.gpio_mem.close();
            self.initialized = false;
        }
    }

    /// Gets the current GPIO pin mode.
    pub fn mode(&self, pin: u8) -> Result<Mode> {
        if !self.initialized {
            return Err(Error::NotInitialized);
        }

        if pin >= GPIO_MAX_PINS {
            return Err(Error::InvalidPin(pin));
        }

        let reg_addr: usize = GPIO_OFFSET_GPFSEL + (pin / 10) as usize;
        let reg_value = self.gpio_mem.read(reg_addr);
        let mode_value: usize = ((reg_value >> ((pin % 10) * 3)) & 0b111) as usize;

        let modes = [
            Mode::Input,
            Mode::Output,
            Mode::Alt5,
            Mode::Alt4,
            Mode::Alt0,
            Mode::Alt1,
            Mode::Alt2,
            Mode::Alt3,
        ];

        if mode_value < modes.len() {
            Ok(modes[mode_value])
        } else {
            Err(Error::UnknownMode(mode_value as u8))
        }
    }

    /// Sets the GPIO pin mode to input, output or one of the alternative functions.
    ///
    /// More information about the alternative functions can be found in the
    /// [`BCM2835`] documentation.
    ///
    /// [`BCM2835`]: https://www.raspberrypi.org/app/uploads/2012/02/BCM2835-ARM-Peripherals.pdf
    pub fn set_mode(&mut self, pin: u8, mode: Mode) {
        if !self.initialized || (pin >= GPIO_MAX_PINS) {
            return;
        }

        // Keep track of our mode changes, so we can revert them in cleanup()
        if let Some(pin_state) = self.orig_pin_state.get_mut(pin as usize) {
            if pin_state.mode != mode {
                pin_state.changed = true;
            }
        }

        let reg_addr: usize = GPIO_OFFSET_GPFSEL + (pin / 10) as usize;

        let reg_value = self.gpio_mem.read(reg_addr);
        self.gpio_mem.write(
            reg_addr,
            (reg_value & !(0b111 << ((pin % 10) * 3)))
                | ((mode as u32 & 0b111) << ((pin % 10) * 3)),
        );
    }

    /// Reads the current GPIO pin logic level.
    pub fn read(&self, pin: u8) -> Result<Level> {
        if !self.initialized {
            return Err(Error::NotInitialized);
        }

        if pin >= GPIO_MAX_PINS {
            return Err(Error::InvalidPin(pin));
        }

        let reg_addr: usize = GPIO_OFFSET_GPLEV + (pin / 32) as usize;
        let reg_value = self.gpio_mem.read(reg_addr);

        if (reg_value & (1 << (pin % 32))) > 0 {
            Ok(Level::High)
        } else {
            Ok(Level::Low)
        }
    }

    /// Changes the GPIO pin logic level to high or low.
    pub fn write(&self, pin: u8, level: Level) {
        if !self.initialized || (pin >= GPIO_MAX_PINS) {
            return;
        }

        let reg_addr: usize = match level {
            Level::Low => GPIO_OFFSET_GPCLR + (pin / 32) as usize,
            Level::High => GPIO_OFFSET_GPSET + (pin / 32) as usize,
        };

        self.gpio_mem.write(reg_addr, 1 << (pin % 32));
    }

    /// Configures the built-in GPIO pull-up/pull-down resistors.
    pub fn set_pullupdown(&self, pin: u8, pud: PullUpDown) {
        if !self.initialized || (pin >= GPIO_MAX_PINS) {
            return;
        }

        // Set the control signal in GPPUD, while leaving the other 30
        // bits unchanged.
        let reg_value = self.gpio_mem.read(GPIO_OFFSET_GPPUD);
        self.gpio_mem.write(
            GPIO_OFFSET_GPPUD,
            (reg_value & !0b11) | ((pud as u32) & 0b11),
        );

        // Set-up time for the control signal.
        sleep(Duration::new(0, 20000)); // >= 20µs

        // Select the first GPPUDCLK register for the first 32 pins, and
        // the second register for the remaining pins.
        let reg_addr: usize = GPIO_OFFSET_GPPUDCLK + (pin / 32) as usize;

        // Clock the control signal into the selected pin.
        self.gpio_mem.write(reg_addr, 1 << (pin % 32));

        // Hold time for the control signal.
        sleep(Duration::new(0, 20000)); // >= 20µs

        // Remove the control signal and clock.
        let reg_value = self.gpio_mem.read(GPIO_OFFSET_GPPUD);
        self.gpio_mem.write(GPIO_OFFSET_GPPUD, reg_value & !0b11);
        self.gpio_mem.write(reg_addr, 0 << (pin % 32));
    }

    /// Configures a synchronous interrupt trigger.
    ///
    /// After configuring a synchronous interrupt trigger, you can use
    /// [`poll_interrupt`] to wait for a trigger event.
    ///
    /// `set_interrupt` will remove any previously configured
    /// (a)synchronous interrupt triggers for the same pin.
    ///
    /// [`poll_interrupt`]: #method.poll_interrupt
    pub fn set_interrupt(&mut self, pin: u8, trigger: Trigger) -> Result<()> {
        if !self.initialized {
            return Err(Error::NotInitialized);
        }

        if pin >= GPIO_MAX_PINS {
            return Err(Error::InvalidPin(pin));
        }

        // We can't have sync and async interrupts on the same pin at the same time
        self.clear_async_interrupt(pin)?;

        // Each pin can only be configured for a single trigger type
        self.sync_interrupts.set_interrupt(pin, trigger)?;

        Ok(())
    }

    /// Removes a previously configured synchronous interrupt trigger.
    pub fn clear_interrupt(&mut self, pin: u8) -> Result<()> {
        if !self.initialized {
            return Err(Error::NotInitialized);
        }

        if pin >= GPIO_MAX_PINS {
            return Err(Error::InvalidPin(pin));
        }

        self.sync_interrupts.clear_interrupt(pin)?;

        Ok(())
    }

    /// Blocks until an interrupt is triggered on the specified pin, or a timeout occurs.
    ///
    /// `poll_interrupt` only works for pins that have been configured for synchronous interrupts using
    /// [`set_interrupt`]. Asynchronous interrupt triggers are automatically polled on a separate thread.
    ///
    /// Setting `reset` to `false` causes `poll_interrupt` to return immediately if the interrupt
    /// has been triggered since the previous call to [`set_interrupt`] or `poll_interrupt`.
    /// Setting `reset` to `true` clears any cached trigger events for the pin.
    ///
    /// The `timeout` duration indicates how long the call to `poll_interrupt` will block while waiting
    /// for interrupt trigger events, after which an `Ok(None))` is returned.
    /// `timeout` can be set to `None` to wait indefinitely.
    ///
    /// [`set_interrupt`]: #method.set_interrupt
    pub fn poll_interrupt(
        &mut self,
        pin: u8,
        reset: bool,
        timeout: Option<Duration>,
    ) -> Result<Option<Level>> {
        match self.poll_interrupts(&[pin], reset, timeout) {
            Ok(opt) => if let Some(trigger) = opt {
                Ok(Some(trigger.1))
            } else {
                Ok(None)
            },
            Err(e) => Err(e),
        }
    }

    /// Blocks until a synchronous interrupt is triggered on any of the specified pins, or a timeout occurs.
    ///
    /// `poll_interrupts` only works for pins that have been configured for synchronous interrupts using
    /// [`set_interrupt`]. Asynchronous interrupt triggers are automatically polled on a separate thread.
    ///
    /// Setting `reset` to `false` causes `poll_interrupts` to return immediately if any of the interrupts
    /// has been triggered since the previous call to [`set_interrupt`] or `poll_interrupts`.
    /// Setting `reset` to `true` clears any cached trigger events for the pins.
    ///
    /// The `timeout` duration indicates how long the call to `poll_interrupts` will block while waiting
    /// for interrupt trigger events, after which an `Ok(None))` is returned.
    /// `timeout` can be set to `None` to wait indefinitely.
    ///
    /// When an interrupt event is triggered, `poll_interrupts` returns
    /// `Ok((u8, Level))` containing the corresponding pin number and logic level. If multiple events trigger
    /// at the same time, only the first one is returned. The remaining events are cached and will be returned
    /// the next time `poll_interrupts` is called.
    ///
    /// [`set_interrupt`]: #method.set_interrupt
    pub fn poll_interrupts(
        &mut self,
        pins: &[u8],
        reset: bool,
        timeout: Option<Duration>,
    ) -> Result<Option<(u8, Level)>> {
        if !self.initialized {
            return Err(Error::NotInitialized);
        }

        for pin in pins {
            if *pin >= GPIO_MAX_PINS {
                return Err(Error::InvalidPin(*pin));
            }
        }

        Ok(self.sync_interrupts.poll(pins, reset, timeout)?)
    }

    /// Configures an asynchronous interrupt trigger, which will execute the callback on a
    /// separate thread when the interrupt is triggered.
    ///
    /// The callback closure or function pointer is called with a single [`Level`] argument.
    ///
    /// `set_async_interrupt` will remove any previously configured
    /// (a)synchronous interrupt triggers for the same pin.
    ///
    /// [`Level`]: enum.Level.html
    pub fn set_async_interrupt<C>(&mut self, pin: u8, trigger: Trigger, callback: C) -> Result<()>
    where
        C: FnMut(Level) + Send + 'static,
    {
        if !self.initialized {
            return Err(Error::NotInitialized);
        }

        if pin >= GPIO_MAX_PINS {
            return Err(Error::InvalidPin(pin));
        }

        // We can't have sync and async interrupts on the same pin at the same time
        self.clear_interrupt(pin)?;

        // Stop and remove existing interrupt trigger on this pin
        self.clear_async_interrupt(pin)?;

        self.async_interrupts[pin as usize] = Some(interrupt::AsyncInterrupt::new(
            self.gpio_cdev.as_raw_fd(),
            pin,
            trigger,
            callback,
        )?);

        Ok(())
    }

    /// Removes a previously configured asynchronous interrupt trigger.
    pub fn clear_async_interrupt(&mut self, pin: u8) -> Result<()> {
        if !self.initialized {
            return Err(Error::NotInitialized);
        }

        if pin >= GPIO_MAX_PINS {
            return Err(Error::InvalidPin(pin));
        }

        if let Some(mut interrupt) = self.async_interrupts[pin as usize].take() {
            // stop() blocks until the poll thread exits
            interrupt.stop()?;
        }

        Ok(())
    }

    pub fn cdev_gpio_poll_interrupt(&self, pin: u8, trigger: Trigger) -> Result<Level> {
        if !self.initialized {
            return Err(Error::NotInitialized);
        }

        if pin >= GPIO_MAX_PINS {
            return Err(Error::InvalidPin(pin));
        }

        Ok(ioctl::poll_interrupt(
            self.gpio_cdev.as_raw_fd(),
            pin,
            trigger,
        )?)
    }
}

impl Drop for Gpio {
    fn drop(&mut self) {
        if self.clear_on_drop {
            self.cleanup();
        }

        unsafe {
            GPIO_INSTANCED.store(false, Ordering::SeqCst);
        }
    }
}

impl fmt::Debug for Gpio {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Gpio")
            .field("initialized", &self.initialized)
            .field("clear_on_drop", &self.clear_on_drop)
            .field("gpio_mem", &self.gpio_mem)
            .field("orig_pin_state", &format_args!("{{ .. }}"))
            .field("sync_interrupts", &format_args!("{{ .. }}"))
            .field("async_interrupts", &format_args!("{{ .. }}"))
            .finish()
    }
}

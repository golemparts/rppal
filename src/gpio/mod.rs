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

//! Interface for the Raspberry Pi's BCM283x GPIO peripheral.
//!
//! The GPIO peripheral interface accesses the appropriate memory registers
//! through either the `/dev/gpiomem` device, or `/dev/mem` for distributions
//! where the former isn't available.
//!
//! On a typical up-to-date Raspbian installation, any user that's part of the
//! `gpio` group can access `/dev/gpiomem`, while `/dev/mem` requires
//! superuser privileges.
//!
//! Pins are addressed by their BCM GPIO pin numbers, rather than their
//! physical location.
//!
//! By default, all pins are reset to their original state when `Gpio` goes out
//! of scope. Use `set_clear_on_drop(false)` to disable this behavior. Note that
//! drop methods aren't called when a program is abnormally terminated (for
//! instance when a SIGINT isn't caught).

#![allow(dead_code)]

use num::FromPrimitive;
use std::error;
use std::fmt;
use std::io;
use std::result;
use std::thread::sleep;
use std::time::Duration;

mod interrupt;
mod mem;
mod sysfs;

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

quick_error! {
    #[derive(Debug)]
/// Errors that can occur when accessing the GPIO peripheral.
    pub enum Error {
/// Invalid GPIO pin number.
///
/// The GPIO pin number is not accessible on this Raspberry Pi model.
        InvalidPin(pin: u8) { description("invalid GPIO pin number") }
/// Unknown GPIO pin mode.
///
/// The GPIO pin is set to an unknown mode.
        UnknownMode(mode: u8) { description("unknown mode") }
/// Unknown SoC.
///
/// Based on the output of `/proc/cpuinfo`, it wasn't possible to identify the Raspberry Pi's SoC.
        UnknownSoC { description("unknown SoC") }
/// Unable to find `/dev/gpiomem` in the filesystem.
///
/// Try upgrading to a more recent version of Raspbian (or
/// equivalent) that implements `/dev/gpiomem`.
        DevGpioMemNotFound { description("/dev/gpiomem not found") }
/// Permission denied when opening `/dev/gpiomem` for read/write access.
///
/// Make sure the user has read and write access to `/dev/gpiomem`.
/// Common causes are either incorrect file permissions on
/// `/dev/gpiomem`, or the user isn't part of the gpio group.
        DevGpioMemPermissionDenied { description("/dev/gpiomem insufficient permissions") }
/// `/dev/gpiomem` IO error.
        DevGpioMemIoError(err: io::Error) {
            description(err.description())
            display("/dev/gpiomem IO error ({})", error::Error::description(err))
            cause(err)
        }
/// Unable to memory-map `/dev/gpiomem`.
        DevGpioMemMapFailed { description("/dev/gpiomem map failed") }
/// Unable to find `/dev/mem` in the filesystem.
        DevMemNotFound { description("/dev/mem not found") }
/// Permission denied when opening `/dev/mem` for read/write access.
///
/// Getting read and write access to `/dev/mem` is typically
/// accomplished by executing the program as a privileged user through
/// `sudo`. A better solution that doesn't require `sudo` would be to
/// upgrade to a version of Raspbian that implements `/dev/gpiomem`.
        DevMemPermissionDenied { description("/dev/mem insufficient permissions") }
/// `/dev/mem` IO error.
        DevMemIoError(err: io::Error) {
            description(err.description())
            display("/dev/mem IO error ({})", error::Error::description(err))
            cause(err)
        }
/// Unable to memory-map `/dev/mem`.
        DevMemMapFailed { description("/dev/mem map failed") }
/// Permission denied when opening both `/dev/gpiomem` and `/dev/mem` for read/write access.
///
/// Make sure the user has read and write access to `/dev/gpiomem`.
/// Common causes are either incorrect file permissions on `/dev/gpiomem`, or
/// the user isn't part of the gpio group.
///
/// Getting read and write access to `/dev/mem` is typically
/// accomplished by executing the program as a privileged user through
/// `sudo`. A better solution that doesn't require `sudo` would be to
/// upgrade to a version of Raspbian that implements `/dev/gpiomem`.
        PermissionDenied { description("/dev/gpiomem and /dev/mem insufficient permissions") }
/// GPIO isn't initialized.
///
/// You should normally only see this error when you call a method after
/// running `cleanup()`.
        NotInitialized { description("not initialized") }
/// Interrupt error.
        Interrupt(err: interrupt::Error) { description(err.description()) from() }
    }
}

/// Result type returned from methods that can have `rppal::gpio::Error`s.
pub type Result<T> = result::Result<T, Error>;

enum_from_primitive! {
    #[derive(Debug, PartialEq, Copy, Clone)]
/// Pin modes.
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

enum_from_primitive! {
    #[derive(Debug, PartialEq, Copy, Clone)]
/// Pin logic levels.
    pub enum Level {
        Low = 0,
        High = 1,
    }
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Level::Low => write!(f, "Low"),
            Level::High => write!(f, "High"),
        }
    }
}

enum_from_primitive! {
    #[derive(Debug, PartialEq, Copy, Clone)]
/// Built-in pull-up/pull-down resistor states.
    pub enum PullUpDown {
        Off = 0b00,
        PullDown = 0b01,
        PullUp = 0b10,
    }
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

/// Interrupt trigger type.
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Trigger {
    Disabled,
    RisingEdge,
    FallingEdge,
    Both,
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
        PinState {
            pin: pin,
            mode: mode,
            changed: changed,
        }
    }
}

/// Provides access to the Raspberry Pi GPIO.
pub struct Gpio {
    initialized: bool,
    clear_on_drop: bool,
    gpio_mem: mem::GpioMem,
    orig_pin_state: Vec<PinState>,
    sync_interrupts: Vec<Option<interrupt::Interrupt>>,
    async_interrupts: interrupt::EventLoop,
}

impl Gpio {
    /// Constructs a new `Gpio`.
    pub fn new() -> Result<Gpio> {
        let mut gpio = Gpio {
            initialized: true,
            clear_on_drop: true,
            gpio_mem: mem::GpioMem::new(),
            orig_pin_state: Vec::with_capacity(GPIO_MAX_PINS as usize),
            sync_interrupts: Vec::with_capacity(GPIO_MAX_PINS as usize),
            async_interrupts: interrupt::EventLoop::new(),
        };

        try!(gpio.gpio_mem.open());

        // Save the original pin states, so we can reset them with cleanup()
        for n in 0..GPIO_MAX_PINS {
            match gpio.mode(n) {
                Ok(mode) => gpio.orig_pin_state.push(PinState::new(n, mode, false)),
                Err(e) => return Err(e),
            }
        }

        // Initialize sync_interrupts while circumventing the Copy/Clone requirement
        for _ in 0..gpio.sync_interrupts.capacity() {
            gpio.sync_interrupts.push(None);
        }

        Ok(gpio)
    }

    /// When enabled, resets all pins to their original state when `Gpio` goes out of scope.
    ///
    /// Drop methods aren't called when a program is abnormally terminated,
    /// for instance when a user presses Ctrl-C, and the SIGINT signal isn't
    /// caught. You'll either have to catch those using crates such as
    /// `simple_signal`, or manually call `cleanup()`.
    ///
    /// Enabled by default.
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
            self.async_interrupts.stop().ok();

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

    /// Reads the current GPIO pin mode.
    pub fn mode(&self, pin: u8) -> Result<Mode> {
        if !self.initialized {
            return Err(Error::NotInitialized);
        }

        if pin >= GPIO_MAX_PINS {
            return Err(Error::InvalidPin(pin));
        }

        let reg_addr: usize = GPIO_OFFSET_GPFSEL + (pin / 10) as usize;
        let reg_value = self.gpio_mem.read(reg_addr);
        let mode_value = (reg_value >> ((pin % 10) * 3)) & 0b111;

        if let Some(mode) = Mode::from_u32(mode_value) {
            Ok(mode)
        } else {
            Err(Error::UnknownMode(mode_value as u8))
        }
    }

    /// Changes the GPIO pin mode to input, output or one of the alternative functions.
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

    /// Enables/disables the built-in GPIO pull-up/pull-down resistors.
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

    /// Setup a synchronous interrupt, and block until the interrupt is triggered, or
    /// a timeout occurs.
    ///
    /// The timeout duration can be set to None to wait indefinitely.
    pub fn poll_interrupt(&mut self, pin: u8, trigger: Trigger, timeout: Option<Duration>, reset: bool) -> Result<Level> {
        if !self.initialized {
            return Err(Error::NotInitialized);
        }

        if pin >= GPIO_MAX_PINS {
            return Err(Error::InvalidPin(pin));
        }

        // Check for a cached interrupt on the same pin
        if let Some(ref mut interrupt) = self.sync_interrupts[pin as usize] {
            // Each pin can only have 1 trigger set
            if interrupt.trigger() != trigger {
                interrupt.set_trigger(trigger)?;
            }

            if reset {
                interrupt.level()?;
            }

            return Ok(interrupt.poll(timeout)?);
        }

        let mut interrupt = interrupt::Interrupt::new(pin, trigger)?;
        let result = interrupt.poll(timeout);

        self.sync_interrupts[pin as usize] = Some(interrupt);

        match result {
            Ok(level) => Ok(level),
            Err(e) => Err(Error::from(e)),
        }
    }

    /// Setup an asynchronous interrupt, which will execute the callback closure or
    /// function pointer when it's triggered.
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

        self.async_interrupts.set_interrupt(pin, trigger, callback)?;

        Ok(())
    }

    /// Remove an existing asynchronous interrupt trigger.
    pub fn clear_async_interrupt(&mut self, pin: u8) -> Result<()> {
        if !self.initialized {
            return Err(Error::NotInitialized);
        }

        if pin >= GPIO_MAX_PINS {
            return Err(Error::InvalidPin(pin));
        }

        self.async_interrupts.clear_interrupt(pin)?;

        Ok(())
    }
}

impl Drop for Gpio {
    fn drop(&mut self) {
        if self.clear_on_drop {
            self.cleanup();
        }
    }
}

// Copyright (c) 2017 Rene van der Meer
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
//! By default, all pins are reset to their original state when `GPIO` goes out
//! of scope. Use `set_clear_on_drop(false)` to disable this behavior. Note that
//! drop methods aren't called when a program is abnormally terminated (for
//! instance when a SIGINT isn't caught).

#![allow(dead_code)]

use std::error;
use std::fmt;
use std::io;
use std::fs::OpenOptions;
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::io::AsRawFd;
use std::result;
use std::thread::sleep;
use std::time::Duration;
use std::ptr;

use libc;
use num::FromPrimitive;

use system::DeviceInfo;

// The BCM2835 has 41 32-bit registers related to the GPIO (datasheet @ 6.1).
const GPIO_MEM_SIZE: usize = 164;
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
        DevGPIOMemNotFound { description("/dev/gpiomem not found") }
/// Permission denied when opening `/dev/gpiomem` for read/write access.
///
/// Make sure the user has read and write access to `/dev/gpiomem`.
/// Common causes are either incorrect file permissions on
/// `/dev/gpiomem`, or the user isn't part of the gpio group.
        DevGPIOMemPermissionDenied { description("/dev/gpiomem insufficient permissions") }
/// `/dev/gpiomem` IO error.
        DevGPIOMemIOError(err: io::Error) {
            description(err.description())
            display("/dev/gpiomem IO error ({})", error::Error::description(err))
            cause(err)
        }
/// Unable to memory-map `/dev/gpiomem`.
        DevGPIOMemMapFailed { description("/dev/gpiomem map failed") }
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
        DevMemIOError(err: io::Error) {
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
    }
}

/// Result type returned from methods that can have `rppal::gpio::Error`s.
pub type Result<T> = result::Result<T, Error>;

struct GPIOMem {
    mapped: bool,
    mem_ptr: *mut u32,
}

impl GPIOMem {
    pub fn new() -> GPIOMem {
        GPIOMem {
            mapped: false,
            mem_ptr: ptr::null_mut(),
        }
    }

    pub fn open(&mut self) -> Result<()> {
        if self.mapped {
            return Ok(());
        }

        // Try /dev/gpiomem first. Report back any errors the user can fix,
        // otherwise try /dev/mem instead.
        self.mem_ptr = match self.map_devgpiomem() {
            Ok(ptr) => ptr,
            Err(gpiomem_err) => {
                match self.map_devmem() {
                    Ok(ptr) => ptr,
                    // Special case when both /dev/gpiomem and /dev/mem have permission issues
                    Err(e @ Error::DevMemPermissionDenied) => {
                        match gpiomem_err {
                            Error::DevGPIOMemPermissionDenied => {
                                return Err(Error::PermissionDenied);
                            }
                            _ => {
                                return Err(e);
                            }
                        }
                    }
                    Err(e) => {
                        return Err(e);
                    }
                }
            }
        };

        self.mapped = true;

        Ok(())
    }

    fn map_devgpiomem(&mut self) -> Result<*mut u32> {
        // Open /dev/gpiomem with read/write/sync flags. This might fail if
        // /dev/gpiomem doesn't exist (< Raspbian Jessie), or /dev/gpiomem
        // doesn't have the appropriate permissions, or the current user is
        // not a member of the gpio group.
        let gpiomem_file = match OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_SYNC)
            .open("/dev/gpiomem") {
            Err(e) => {
                match e.kind() {
                    io::ErrorKind::NotFound => return Err(Error::DevGPIOMemNotFound),
                    io::ErrorKind::PermissionDenied => {
                        return Err(Error::DevGPIOMemPermissionDenied)
                    }
                    _ => return Err(Error::DevGPIOMemIOError(e)),
                }
            }
            Ok(file) => file,
        };

        // Memory-map /dev/gpiomem at offset 0
        let gpiomem_ptr = unsafe {
            libc::mmap(
                ptr::null_mut(),
                GPIO_MEM_SIZE,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                gpiomem_file.as_raw_fd(),
                0,
            )
        };

        if gpiomem_ptr == libc::MAP_FAILED {
            return Err(Error::DevGPIOMemMapFailed);
        }

        Ok(gpiomem_ptr as *mut u32)
    }

    fn map_devmem(&mut self) -> Result<*mut u32> {
        // Identify which SoC we're using, so we know what offset to start at
        let device_info = match DeviceInfo::new() {
            Ok(s) => s,
            Err(_) => return Err(Error::UnknownSoC),
        };

        let mem_file = match OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_SYNC)
            .open("/dev/mem") {
            Err(e) => {
                match e.kind() {
                    io::ErrorKind::NotFound => return Err(Error::DevMemNotFound),
                    io::ErrorKind::PermissionDenied => return Err(Error::DevMemPermissionDenied),
                    _ => return Err(Error::DevMemIOError(e)),
                }
            }
            Ok(file) => file,
        };

        // Memory-map /dev/mem at the appropriate offset for our SoC
        let mem_ptr = unsafe {
            libc::mmap(
                ptr::null_mut(),
                GPIO_MEM_SIZE,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                mem_file.as_raw_fd(),
                (device_info.peripheral_base() + device_info.gpio_offset()) as libc::off_t,
            )
        };

        if mem_ptr == libc::MAP_FAILED {
            return Err(Error::DevMemMapFailed);
        }

        Ok(mem_ptr as *mut u32)
    }

    pub fn close(&mut self) {
        if !self.mapped {
            return;
        }

        unsafe {
            libc::munmap(
                self.mem_ptr as *mut libc::c_void,
                GPIO_MEM_SIZE as libc::size_t,
            );
        }

        self.mapped = false;
    }

    pub fn read(&mut self, offset: usize) -> u32 {
        if !self.mapped || offset >= GPIO_MEM_SIZE {
            return 0;
        }

        unsafe { ptr::read_volatile(self.mem_ptr.offset(offset as isize)) }
    }

    pub fn write(&self, offset: usize, value: u32) {
        if !self.mapped || offset >= GPIO_MEM_SIZE {
            return;
        }

        unsafe {
            ptr::write_volatile(self.mem_ptr.offset(offset as isize), value);
        }
    }
}

impl Drop for GPIOMem {
    fn drop(&mut self) {
        self.close();
    }
}

// Required because of the raw pointer to our memory-mapped file
unsafe impl Send for GPIOMem {}

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
pub struct GPIO {
    initialized: bool,
    clear_on_drop: bool,
    gpio_mem: GPIOMem,
    orig_pin_state: Vec<PinState>,
}

impl GPIO {
    /// Constructs a new `GPIO`.
    pub fn new() -> Result<GPIO> {
        let mut gpio = GPIO {
            initialized: true,
            clear_on_drop: true,
            gpio_mem: GPIOMem::new(),
            orig_pin_state: Vec::with_capacity(GPIO_MAX_PINS as usize),
        };

        try!(gpio.gpio_mem.open());

        // Save the original pin states, so we can reset them with cleanup()
        for n in 0..GPIO_MAX_PINS {
            match gpio.mode(n) {
                Ok(mode) => gpio.orig_pin_state.push(PinState::new(n, mode, false)),
                Err(e) => return Err(e),
            }
        }

        Ok(gpio)
    }

    /// When enabled, resets all pins to their original state when `GPIO` goes out of scope.
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
    /// Normally, this method is automatically called when `GPIO` goes out of
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

    /// Reads the current GPIO pin mode.
    pub fn mode(&mut self, pin: u8) -> Result<Mode> {
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
            (reg_value & !(0b111 << ((pin % 10) * 3))) |
                ((mode as u32 & 0b111) << ((pin % 10) * 3)),
        );
    }

    /// Reads the current GPIO pin logic level.
    pub fn read(&mut self, pin: u8) -> Result<Level> {
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
    pub fn set_pullupdown(&mut self, pin: u8, pud: PullUpDown) {
        if !self.initialized || (pin >= GPIO_MAX_PINS) {
            return;
        }

        let reg_addr: usize = GPIO_OFFSET_GPPUDCLK + (pin / 32) as usize;
        let reg_value = self.gpio_mem.read(GPIO_OFFSET_GPPUD);
        self.gpio_mem.write(
            GPIO_OFFSET_GPPUD,
            (reg_value & !0b11) | ((pud as u32) & 0b11),
        );

        sleep(Duration::new(0, 20000)); // 20µs

        self.gpio_mem.write(reg_addr, 1 << (pin % 32));

        sleep(Duration::new(0, 20000)); // 20µs

        let reg_value = self.gpio_mem.read(GPIO_OFFSET_GPPUD);
        self.gpio_mem.write(GPIO_OFFSET_GPPUD, (reg_value & !0b11));
        self.gpio_mem.write(reg_addr, 0 << (pin % 32));
    }
}

impl Drop for GPIO {
    fn drop(&mut self) {
        if self.clear_on_drop {
            self.cleanup();
        }
    }
}

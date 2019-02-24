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

//! Interface for the UART peripherals and USB serial devices.
//!
//! RPPAL controls the Raspberry Pi's PL011 and mini UART peripherals
//! through the `ttyAMA0` and `ttyS0` device interfaces. Communicating with
//! USB serial devices is supported through `ttyUSBx` and `ttyACMx`.
//!
//! ## UART peripherals
//!
//! The Raspberry Pi's BCM283x SoC features two UART peripherals.
//! `/dev/ttyAMA0` represents the primary (PL011) UART, which offers a full
//! set of features. `/dev/ttyS0` represents an auxiliary peripheral that's
//! referred to as mini UART, with limited capabilities.
//!
//! On earlier Raspberry Pi models without Bluetooth, `/dev/ttyAMA0` is configured
//! as a Linux serial console. On more recent models with Bluetooth (3A+, 3B,
//! 3B+, Zero W), `/dev/ttyAMA0` is connected to the Bluetooth module, and `/dev/ttyS0`
//! is used as a serial console instead. Due to the limitations of `/dev/ttyS0` and
//! the requirement for a fixed core frequency, in most cases you'll want to
//! use `/dev/ttyAMA0` for serial communication. More details on the differences
//! between `/dev/ttyAMA0` and `/dev/ttyS0` can be found in the official Raspberry Pi
//! [documentation].
//!
//! By default, TX (outgoing data) is tied to BCM GPIO 14 (physical pin 8) and
//! RX (incoming data) is tied to BCM GPIO 15 (physical pin 10). You can move
//! these lines to different pins using the `uart0` and `uart1` overlays,
//! however none of the other pin options are exposed through the GPIO header
//! on any of the current Raspberry Pi models. They are only available on the
//! Compute Module's SO-DIMM pads.
//!
//! ## Configure `/dev/ttyAMA0` for serial communication (recommended)
//!
//! To disable the Linux serial console, either deactivate it through
//! `sudo raspi-config`, or remove the parameter `console=serial0,115200` from
//! `/boot/cmdline.txt`.
//!
//! Remove any lines containing `enable_uart=0` or `enable_uart=1` from
//! `/boot/config.txt`.
//!
//! On Raspberry Pi models with Bluetooth, an extra step is required to either
//! disable Bluetooth so `/dev/ttyAMA0` becomes available for serial communication,
//! or tie the Bluetooth module to `/dev/ttyS0`.
//!
//! To disable Bluetooth, add `dtoverlay=pi3-disable-bt` to `/boot/config.txt`.
//! You'll also need to disable the service that initializes Bluetooth with
//! `sudo systemctl disable hciuart`.
//!
//! To move the Bluetooth module to `/dev/ttyS0`, instead of the above-mentioned
//! steps, add `dtoverlay=pi3-miniuart-bt` to `/boot/config.txt`. You'll also
//! need to edit `/lib/systemd/system/hciuart.service` and replace `/dev/ttyAMA0`
//! with `/dev/ttyS0`, and set a fixed core frequency by adding `core_freq=250` to
//! `/boot/config.txt`.
//!
//! Remember to reboot the Raspberry Pi after making any changes.
//!
//! ## Configure `/dev/ttyS0` for serial communication
//!
//! If you prefer to leave the Bluetooth module on `/dev/ttyAMA0`, you can configure
//! `/dev/ttyS0` for serial communication instead.
//!
//! To disable the Linux serial console, either deactivate it through
//! `sudo raspi-config`, or remove the parameter `console=serial0,115200` from
//! `/boot/cmdline.txt`.
//!
//! Add the line `enable_uart=1` to `/boot/config.txt` to enable serial
//! communication on `/dev/ttyS0`, which also sets a fixed core frequency.
//!
//! Remember to reboot the Raspberry Pi after making any changes.
//!
//! ## Hardware flow control
//!
//! RTS (request to send) is tied to BCM GPIO 17 (physical pin 11) and
//! CTS (clear to send) is tied to BCM GPIO 16 (physical pin 36). Enabling
//! hardware flow control with [`set_hardware_flow_control`] will automatically
//! configure these pins.
//!
//! The RTS and CTS pins are reset to their original state when [`Uart`] goes
//! out of scope. Note that `drop` methods aren't called when a process is
//! abnormally terminated, for instance when a user presses <kbd>Ctrl</kbd> +
//! <kbd>C</kbd>, and the `SIGINT` signal isn't caught. You can catch those
//! using crates such as [`simple_signal`].
//!
//! ## USB serial devices
//!
//! In addition to the hardware UART peripherals, `Uart` can also control USB
//! serial devices. Depending on the type of device/USB controller chip,
//! these can be accessed either through `/dev/ttyUSBx` or `/dev/ttyACMx`,
//! where `x` is an index starting at `0`. The numbering is based on the order
//! in which the devices are discovered by the kernel. You'll need to find
//! a way to uniquely identify them when you have multiple devices connected
//! at the same time, for instance, by searching for the relevant name in
//! the `/dev/serial/by-id` directory.
//!
//! ## Troubleshooting
//!
//! ### Permission denied
//!
//! The current user should be a member of the group that owns the specified
//! device. Usually the group is set to either `dialout` or `tty`.
//!
//! [documentation]: https://www.raspberrypi.org/documentation/configuration/uart.md
//! [`simple_signal`]: https://crates.io/crates/simple-signal

use std::error;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io;
use std::io::{Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::io::{AsRawFd, RawFd};
use std::path::Path;
use std::result;
use std::time::Duration;

use libc::O_NOCTTY;

use crate::gpio::{self, Gpio, IoPin, Mode};

#[cfg(feature = "hal")]
mod hal;
mod termios;

const UART_RTS_GPIO: u8 = 17;
const UART_CTS_GPIO: u8 = 16;

const UART0_RTS_MODE: Mode = Mode::Alt3;
const UART0_CTS_MODE: Mode = Mode::Alt3;

const UART1_RTS_MODE: Mode = Mode::Alt5;
const UART1_CTS_MODE: Mode = Mode::Alt5;

/// Errors that can occur when accessing the UART peripheral.
#[derive(Debug)]
pub enum Error {
    /// I/O error.
    Io(io::Error),
    /// GPIO error.
    Gpio(gpio::Error),
    /// Invalid value.
    InvalidValue,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Error::Io(ref err) => write!(f, "I/O error: {}", err),
            Error::Gpio(ref err) => write!(f, "GPIO error: {}", err),
            Error::InvalidValue => write!(f, "Invalid value"),
        }
    }
}

impl error::Error for Error {}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}

impl From<gpio::Error> for Error {
    fn from(err: gpio::Error) -> Error {
        Error::Gpio(err)
    }
}

/// Result type returned from methods that can have `uart::Error`s.
pub type Result<T> = result::Result<T, Error>;

/// Parity modes.
///
/// `None` omits the parity bit. `Even` and `Odd` count the total number of
/// 1-bits in the data bits. `Mark` and `Space` always set the parity
/// bit to `1` or `0` respectively.
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Parity {
    None,
    Even,
    Odd,
    Mark,
    Space,
}

/// Buffer types.
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Buffer {
    Incoming,
    Outgoing,
    Both,
}

/// Provides access to the Raspberry Pi's UART peripherals and any connected USB serial
/// devices.
#[derive(Debug)]
pub struct Uart {
    device: File,
    fd: RawFd,
    rts_cts_mode: Option<(Mode, Mode)>,
    rts_cts: Option<(IoPin, IoPin)>,
}

impl Uart {
    /// Constructs a new `Uart`.
    ///
    /// `new` attempts to identify the UART peripheral tied to BCM GPIO 14 and 15, and
    /// then calls [`with_path`] with the appropriate device path.
    ///
    /// [`with_path`]: #method.with_path
    pub fn new(line_speed: u32, parity: Parity, data_bits: u8, stop_bits: u8) -> Result<Uart> {
        Self::with_path("/dev/serial0", line_speed, parity, data_bits, stop_bits)
    }

    /// Constructs a new `Uart` connected to the serial device specified by `path`.
    ///
    /// `with_path` can be used to connect to either a UART peripheral or a USB serial device.
    ///
    /// When a new `Uart` is constructed, the specified device is configured for
    /// non-canonical mode which processes input per character, ignores any special
    /// terminal input or output characters and disables local echo.
    pub fn with_path<P: AsRef<Path>>(
        path: P,
        line_speed: u32,
        parity: Parity,
        data_bits: u8,
        stop_bits: u8,
    ) -> Result<Uart> {
        // Follow symbolic links
        let path = fs::canonicalize(path)?;

        // Check if we're using /dev/ttyAMA0 or /dev/ttyS0 so we can set the correct
        // RTS/CTS pin modes when needed.
        let rts_cts_mode = if let Some(path_str) = path.to_str() {
            match path_str {
                "/dev/ttyAMA0" => Some((UART0_RTS_MODE, UART0_CTS_MODE)),
                "/dev/ttyS0" => Some((UART1_RTS_MODE, UART1_CTS_MODE)),
                _ => None,
            }
        } else {
            None
        };

        // While it's tempting to set O_NONBLOCK here to prevent write()
        // from blocking, that also prevents read() from working properly
        // with the VMIN and VTIME settings.
        let device = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(O_NOCTTY)
            .open(path)?;

        let fd = device.as_raw_fd();

        // Enables character input mode, disables echoing and any special processing
        termios::set_raw_mode(fd)?;

        // Non-blocking reads
        termios::set_read_mode(fd, 0, Duration::default())?;

        // Ignore modem control lines (CLOCAL)
        termios::ignore_carrier_detect(fd)?;

        // Enable receiver (CREAD)
        termios::enable_read(fd)?;

        // Disable software flow control (XON/XOFF)
        termios::set_software_flow_control(fd, false)?;

        // Disable hardware flow control (RTS/CTS)
        termios::set_hardware_flow_control(fd, false)?;

        termios::set_line_speed(fd, line_speed)?;
        termios::set_parity(fd, parity)?;
        termios::set_data_bits(fd, data_bits)?;
        termios::set_stop_bits(fd, stop_bits)?;

        // Flush the incoming and outgoing buffer
        termios::flush(fd, Buffer::Both)?;

        Ok(Uart {
            device,
            fd,
            rts_cts_mode,
            rts_cts: None,
        })
    }

    /// Returns the line speed in bits per second (bit/s).
    pub fn line_speed(&self) -> Result<u32> {
        termios::line_speed(self.fd)
    }

    /// Sets the line speed in bits per second (bit/s).
    ///
    /// Accepted values:
    /// `0`, `50`, `75`, `110`, `134`, `150`, `200`, `300`, `600`,
    /// `1_200`, `1_800`, `2_400`, `4_800`, `9_600`, `19_200`,
    /// `38_400`, `57_600`, `115_200`, `230_400`, `460_800`, `500_000`,
    /// `576_000`, `921_600`, `1_000_000`, `1_152_000`, `1_500_000`,
    /// `2_000_000`, `2_500_000`, `3_000_000`, `3_500_000`, `4_000_000`.
    ///
    /// Support for some values may be device-dependent.
    pub fn set_line_speed(&self, line_speed: u32) -> Result<()> {
        termios::set_line_speed(self.fd, line_speed)
    }

    /// Returns the parity bit mode.
    pub fn parity(&self) -> Result<Parity> {
        termios::parity(self.fd)
    }

    /// Sets the parity bit mode.
    ///
    /// Support for some modes may be device-dependent.
    pub fn set_parity(&self, parity: Parity) -> Result<()> {
        termios::set_parity(self.fd, parity)
    }

    /// Returns the number of data bits.
    pub fn data_bits(&self) -> Result<u8> {
        termios::data_bits(self.fd)
    }

    /// Sets the number of data bits.
    ///
    /// Accepted values: `5`, `6`, `7`, `8`.
    ///
    /// Support for some values may be device-dependent.
    pub fn set_data_bits(&self, data_bits: u8) -> Result<()> {
        termios::set_data_bits(self.fd, data_bits)
    }

    /// Returns the number of stop bits.
    pub fn stop_bits(&self) -> Result<u8> {
        termios::stop_bits(self.fd)
    }

    /// Sets the number of stop bits.
    ///
    /// Accepted values: `1`, `2`.
    ///
    /// Support for some values may be device-dependent.
    pub fn set_stop_bits(&self, stop_bits: u8) -> Result<()> {
        termios::set_stop_bits(self.fd, stop_bits)
    }

    /// Returns the status of the RTS/CTS hardware flow control setting.
    pub fn hardware_flow_control(&self) -> Result<bool> {
        termios::hardware_flow_control(self.fd)
    }

    /// Enables or disables RTS/CTS hardware flow control.
    ///
    /// If `Uart` is controlling a UART peripheral, enabling
    /// hardware flow control will also configure the appropriate GPIO pins.
    ///
    /// By default, hardware flow control is disabled.
    ///
    /// Support for RTS/CTS is device-dependent. More information on the GPIO
    /// pin numbers associated with the RTS and CTS lines can be found [here].
    ///
    /// [here]: index.html
    pub fn set_hardware_flow_control(&mut self, enabled: bool) -> Result<()> {
        if enabled && self.rts_cts.is_none() {
            // Configure and store RTS/CTS GPIO pins for UART0/UART1, so their
            // mode is automatically reset when Uart goes out of scope.
            if let Some((rts_mode, cts_mode)) = self.rts_cts_mode {
                let gpio = Gpio::new()?;
                let pin_rts = gpio.get(UART_RTS_GPIO)?.into_io(rts_mode);
                let pin_cts = gpio.get(UART_CTS_GPIO)?.into_io(cts_mode);

                self.rts_cts = Some((pin_rts, pin_cts));
            }
        } else if !enabled {
            self.rts_cts = None;
        }

        termios::set_hardware_flow_control(self.fd, enabled)
    }

    /// Returns `true` if CTS (clear to send) is asserted.
    pub fn cts(&self) -> Result<bool> {
        termios::cts(self.fd)
    }

    /// Returns `true` if RTS (request to send) is asserted.
    pub fn rts(&self) -> Result<bool> {
        termios::rts(self.fd)
    }

    /// Asserts or releases the RTS (request to send) line.
    ///
    /// Setting RTS has no effect when [`hardware_flow_control`]
    /// is disabled.
    ///
    /// [`hardware_flow_control`]: #method.hardware_flow_control
    pub fn set_rts(&self, enabled: bool) -> Result<()> {
        termios::set_rts(self.fd, enabled)
    }

    /// Returns a tuple containing the configured `min_length` and `timeout` values.
    pub fn blocking_mode(&self) -> Result<(usize, Duration)> {
        termios::read_mode(self.fd)
    }

    /// Sets the blocking mode for subsequent calls to [`read`].
    ///
    /// `min_length` indicates the minimum number of requested bytes. This value
    /// may differ from the actual buffer length. Maximum value: 255 bytes.
    ///
    /// `timeout` indicates how long the `read` call will block while waiting
    /// for incoming data. `timeout` uses a 0.1 second resolution. Maximum value: 25.5 seconds.
    ///
    /// `read` operates in one of four modes, depending on the specified `min_length` and `timeout`:
    ///
    /// * **Non-blocking read** (`min_length` = 0, `timeout` = 0). `read` stores any available data and
    /// returns immediately.
    /// * **Blocking read** (`min_length` > 0, `timeout` = 0). `read` blocks until at least
    /// `min_length` bytes are available, or the provided buffer variable is full.
    /// * **Read with timeout** (`min_length` = 0, `timeout` > 0). `read` blocks until at least
    /// one byte is available, or the `timeout` duration elapses.
    /// * **Read with inter-byte timeout** (`min_length` > 0, `timeout` > 0). `read` blocks until at least
    /// `min_length` bytes are available, the provided buffer variable is full, or the `timeout`
    /// duration elapses after receiving one or more bytes. The timer is started after an initial byte
    /// becomes available, and is restarted after each additional byte. That means `read` will block
    /// indefinitely until at least one byte is available.
    ///
    /// By default, `read` is configured for non-blocking reads.
    ///
    /// [`read`]: #method.read
    pub fn set_blocking_mode(&self, min_length: usize, timeout: Duration) -> Result<()> {
        termios::set_read_mode(self.fd, min_length, timeout)?;

        Ok(())
    }

    /// Receives incoming data from the device and stores it in `buffer`.
    ///
    /// `read` operates in one of four (non)blocking modes, depending on the settings configured by
    /// [`set_blocking_mode`].
    ///
    /// Returns how many bytes were read.
    ///
    /// [`set_blocking_mode`]: #method.set_blocking_mode
    pub fn read(&mut self, buffer: &mut [u8]) -> Result<usize> {
        match self.device.read(buffer) {
            Ok(bytes_read) => Ok(bytes_read),
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => Ok(0),
            Err(e) => Err(Error::Io(e)),
        }
    }

    /// Sends the contents of `buffer` to the device.
    ///
    /// `write` returns immediately after copying the contents of `buffer`
    /// to the internal outgoing buffer. If the internal buffer is full,
    /// `write` blocks until the entire contents of `buffer` can be copied.
    ///
    /// You can call [`drain`] to wait until all data stored in the internal
    /// outgoing buffer has been transmitted.
    ///
    /// Returns how many bytes were written.
    ///
    /// [`drain`]: #method.drain
    pub fn write(&mut self, buffer: &[u8]) -> Result<usize> {
        match self.device.write(buffer) {
            Ok(bytes_written) => Ok(bytes_written),
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => Ok(0),
            Err(e) => Err(Error::Io(e)),
        }
    }

    /// Blocks until all waiting outgoing data has been transmitted.
    pub fn drain(&self) -> Result<()> {
        termios::drain(self.fd)
    }

    /// Discards all waiting data in the internal incoming and/or outgoing buffer.
    pub fn flush(&self, buffer_type: Buffer) -> Result<()> {
        termios::flush(self.fd, buffer_type)
    }
}

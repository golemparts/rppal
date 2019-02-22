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
//! RPPAL controls the Raspberry Pi's main and auxiliary UART peripherals
//! through the `ttyAMA0` and `ttyS0` device interfaces. In addition to the built-in
//! UARTs, communicating with USB serial devices is supported through `ttyUSBx`
//! and `ttyACMx`.
//!
//! ## UART peripherals
//!
//! On earlier Pi models without Bluetooth, UART0 is used as a Linux serial console
//! if that feature is enabled. On more recent models with Bluetooth (3B, 3B+, Zero W), UART0
//! is connected to the Bluetooth module, and UART1 is used as a serial console if enabled.
//! Due to the limitations of UART1, in most cases you'll want to use UART0 for serial
//! communication. More details on the differences between UART0 and UART1 can be found
//! in the official Raspberry Pi [documentation].
//!
//! To disable the serial console, either deactivate it through `sudo raspi-config`, or
//! remove the line `enable_uart=1` from `/boot/config.txt`. You'll also want to remove
//! the parameter `console=serial0,115200` from `/boot/cmdline.txt`.
//!
//! On Pi models with Bluetooth, an extra step is required to either disable Bluetooth so
//! UART0 becomes available for serial communication, or tie UART1 to the Bluetooth module
//! instead of UART0.
//!
//! To disable Bluetooth, add 'dtoverlay=pi3-disable-bt' to `/boot/config.txt`. You'll also
//! need to disable the service that initializes the modem with `sudo systemctl disable hciuart`.
//!
//! To move the Bluetooth module to UART1, instead of the above-mentioned steps, add
//! `dtoverlay=pi3-miniuart-bt` to `/boot/config.txt`. You'll also need to edit `/lib/systemd/system/hciuart.service`
//! and replace `ttyAMA0` with `ttyS0`, and set a fixed core frequency by adding `core_freq=250` to
//! `/boot/config.txt`.
//!
//! By default, TX (outgoing data) for both UARTs is configured as BCM GPIO 14 (physical pin 8). RX (incoming data) for
//! both UARTs is configured as BCM GPIO 15 (physical pin 10). You can move these to different pins using the `uart0`
//! and `uart1` overlays, however none of the other pin options are exposed through the GPIO header on any of the
//! current Raspberry Pi models. They are only available through the Compute Module's SO-DIMM pads.
//!
//! Remember to reboot the Raspberry Pi after making any changes. More information on `enable_uart`, `pi3-disable-bt`,
//! `pi3-miniuart-bt`, `uart0` and `uart1` can be found in `/boot/overlays/README`.
//!
//! ### UART0 (`/dev/ttyAMA0`)
//!
//! PL011
//!
//! * TX: BCM GPIO 14 Alt0 (physical pin 8)
//! * RX: BCM GPIO 15 Alt0 (physical pin 10)
//! * CTS: BCM GPIO 16 Alt3 (physical pin 36)
//! * RTS: BCM GPIO 17 Alt3 (physical pin 11)
//!
//! ### UART1 (`/dev/ttyS0`)
//!
//! Mini UART
//!
//! * TX: BCM GPIO 14 Alt5 (physical pin 8)
//! * RX: BCM GPIO 15 Alt5 (physical pin 10)
//! * CTS: BCM GPIO 16 Alt5 (physical pin 36)
//! * RTS: BCM GPIO 17 Alt5 (physical pin 11)
//!
//! ## USB serial devices
//!
//! ## Troubleshooting
//!
//! ### Permission denied
//!
//! [documentation]: https://www.raspberrypi.org/documentation/configuration/uart.md

use std::error;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io;
use std::io::{Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::io::AsRawFd;
use std::path::Path;
use std::result;
use std::time::Duration;

use libc::O_NOCTTY;

mod termios;

/// Errors that can occur when accessing the UART peripheral.
#[derive(Debug)]
pub enum Error {
    /// I/O error.
    Io(io::Error),
    /// Invalid value.
    InvalidValue,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Error::Io(ref err) => write!(f, "I/O error: {}", err),
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

/// Result type returned from methods that can have `uart::Error`s.
pub type Result<T> = result::Result<T, Error>;

/// Serial devices.
///
/// The BCM283x SoC includes two UARTs. `Uart0` is the primary (PL011)
/// UART, which offers a full set of features. `Uart1` is an auxiliary
/// peripheral that's referred to as mini UART, with limited capabilities.
/// More information on the differences between the two UARTs, and how to
/// enable them, can be found [here].
///
/// In addition to the built-in UARTs, `Uart` also supports USB devices
/// with a serial interface. Depending on the type of device, these
/// can be accessed either through `/dev/ttyUSBx` or `/dev/ttyACMx`, where `x`
/// is an index starting at `0`. The numbering is based on the order
/// in which the devices are discovered by the kernel, so you'll need to find
/// a way to uniquely identify them when you have multiple devices connected
/// at the same time. For instance, you can find the assigned tty device name
/// based on the device id in `/dev/serial/by-id`.
///
/// [here]: index.html
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Device {
    Uart0,
    Uart1,
    Acm(u8),
    Usb(u8),
}

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

/// Provides access to the Raspberry Pi's UART peripherals and any USB serial devices.
#[derive(Debug)]
pub struct Uart {
    device: File,
}

impl Uart {
    /// Constructs a new `Uart`.
    pub fn new(
        device: Device,
        line_speed: u32,
        parity: Parity,
        data_bits: u8,
        stop_bits: u8,
    ) -> Result<Uart> {
        Self::with_path(
            match device {
                Device::Uart0 => "/dev/ttyAMA0".to_owned(),
                Device::Uart1 => "/dev/ttyS0".to_owned(),
                Device::Acm(idx) => format!("/dev/ttyACM{}", idx),
                Device::Usb(idx) => format!("/dev/ttyUSB{}", idx),
            },
            line_speed,
            parity,
            data_bits,
            stop_bits,
        )
    }

    /// Constructs a new `Uart` connected to the serial device specified by `path`.
    pub fn with_path<P: AsRef<Path>>(
        path: P,
        line_speed: u32,
        parity: Parity,
        data_bits: u8,
        stop_bits: u8,
    ) -> Result<Uart> {
        let device = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(O_NOCTTY)
            .open(path)?;

        // Enables character input mode, disables echoing and any special processing,
        // sets read() to non-blocking (VMIN=0) and timeout to 0 (VTIME=0).
        termios::set_raw_mode(device.as_raw_fd())?;

        // Non-blocking reads
        termios::configure_read(device.as_raw_fd(), 0, Duration::default())?;

        // Ignore modem control lines (CLOCAL)
        termios::ignore_carrier_detect(device.as_raw_fd())?;

        // Enable receiver (CREAD)
        termios::enable_read(device.as_raw_fd())?;

        // Disable software flow control (XON/XOFF)
        termios::set_software_flow_control(device.as_raw_fd(), false)?;

        // Disable hardware flow control (RTS/CTS)
        termios::set_hardware_flow_control(device.as_raw_fd(), false)?;

        termios::set_line_speed(device.as_raw_fd(), line_speed)?;
        termios::set_parity(device.as_raw_fd(), parity)?;
        termios::set_data_bits(device.as_raw_fd(), data_bits)?;
        termios::set_stop_bits(device.as_raw_fd(), stop_bits)?;

        // Flush the incoming and outgoing buffer
        termios::flush(device.as_raw_fd())?;

        Ok(Uart { device })
    }

    /// Returns the line speed in baud (Bd).
    pub fn line_speed(&self) -> Result<u32> {
        termios::line_speed(self.device.as_raw_fd())
    }

    /// Sets the line speed in baud (Bd).
    ///
    /// Valid values are
    /// 0, 50, 75, 110, 134, 150, 200, 300, 600, 1_200, 1_800, 2_400, 4_800,
    /// 9_600, 19_200, 38_400, 57_600, 115_200, 230_400, 460_800, 500_000,
    /// 576_000, 921_600, 1_000_000, 1_152_000, 1_500_000, 2_000_000,
    /// 2_500_000, 3_000_000, 3_500_000 and 4_000_000,
    /// but support is device-dependent.
    pub fn set_line_speed(&self, line_speed: u32) -> Result<()> {
        termios::set_line_speed(self.device.as_raw_fd(), line_speed)
    }

    /// Returns the parity bit.
    pub fn parity(&self) -> Result<Parity> {
        termios::parity(self.device.as_raw_fd())
    }

    /// Sets the parity bit.
    ///
    /// Support for parity is device-dependent.
    pub fn set_parity(&self, parity: Parity) -> Result<()> {
        termios::set_parity(self.device.as_raw_fd(), parity)
    }

    /// Returns the number of data bits.
    pub fn data_bits(&self) -> Result<u8> {
        termios::data_bits(self.device.as_raw_fd())
    }

    /// Sets the number of data bits.
    ///
    /// Valid values are 5, 6, 7 or 8, but support is device-dependent.
    pub fn set_data_bits(&self, data_bits: u8) -> Result<()> {
        termios::set_data_bits(self.device.as_raw_fd(), data_bits)
    }

    /// Gets the number of stop bits.
    pub fn stop_bits(&self) -> Result<u8> {
        termios::stop_bits(self.device.as_raw_fd())
    }

    /// Sets the number of stop bits.
    ///
    /// Valid values are 1 or 2, but support is device-dependent.
    pub fn set_stop_bits(&self, stop_bits: u8) -> Result<()> {
        termios::set_stop_bits(self.device.as_raw_fd(), stop_bits)
    }

    /// Returns the status of the RTS/CTS hardware flow control setting.
    pub fn hardware_flow_control(&self) -> Result<bool> {
        termios::hardware_flow_control(self.device.as_raw_fd())
    }

    /// Enables or disables RTS/CTS hardware flow control.
    ///
    /// Enabling hardware flow control will configure the corresponding GPIO
    /// pins. By default, hardware flow control is disabled.
    ///
    /// Support for RTS/CTS is device-dependent. More information on the GPIO
    /// pin numbers associated with the RTS and CTS lines can be found [here].
    ///
    /// [here]: index.html
    pub fn set_hardware_flow_control(&self, enabled: bool) -> Result<()> {
        termios::set_hardware_flow_control(self.device.as_raw_fd(), enabled)
    }

    /// Receives incoming data from the device and stores it in `buffer`.
    ///
    /// `read` immediately returns after storing any available incoming data,
    /// which could be 0 bytes. Use [`poll`] instead for situations where the
    /// call should block while waiting for data.
    ///
    /// Returns how many bytes were read.
    ///
    /// [`poll`]: #method.poll
    pub fn read(&mut self, buffer: &mut [u8]) -> Result<usize> {
        match self.device.read(buffer) {
            Ok(bytes_read) => Ok(bytes_read),
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => Ok(0),
            Err(e) => Err(Error::Io(e)),
        }
    }

    /// Sends the contents of `buffer` to the device.
    ///
    /// Returns how many bytes were written.
    pub fn write(&mut self, buffer: &[u8]) -> Result<usize> {
        match self.device.write(buffer) {
            Ok(bytes_written) => Ok(bytes_written),
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => Ok(0),
            Err(e) => Err(Error::Io(e)),
        }
    }

    /// Blocks while waiting for incoming data, and then stores it in `buffer`.
    ///
    /// `min_length` indicates the minimum number of requested bytes. This value
    /// may differ from the actual `buffer` length. Maximum value: 255 bytes.
    ///
    /// `timeout` indicates how long the call will block while waiting
    /// for incoming data. `timeout` uses a 0.1 second resolution. Maximum value: 25.5 seconds.
    ///
    /// `poll` operates in one of four modes, depending on the specified arguments:
    ///
    /// * **Non-blocking read** (`min_length` = 0, `timeout` = 0). `poll` behaves similarly to [`read`].
    /// * **Blocking read** (`min_length` > 0, `timeout` = 0). `poll` blocks until at least
    /// `min_length` bytes are available.
    /// * **Read with timeout** (`min_length` = 0, `timeout` > 0). `poll` blocks until at least
    /// 1 byte is available, or the `timeout` duration elapses.
    /// * **Read with inter-byte timeout** (`min_length` > 0, `timeout` > 0). `poll` blocks until at least
    /// `min_length` bytes are available, or the `timeout` duration elapses after receiving 1 or more bytes.
    /// The timer is started after an initial byte becomes available, and is restarted after each additional
    /// byte. That means `poll` will block indefinitely until at least 1 byte is available.
    ///
    /// Returns how many bytes were read.
    ///
    /// [`read`]: #method.read
    pub fn poll(
        &mut self,
        buffer: &mut [u8],
        min_length: usize,
        timeout: Duration,
    ) -> Result<usize> {
        // Configure read
        termios::configure_read(self.device.as_raw_fd(), min_length, timeout)?;

        let return_value = match self.device.read(buffer) {
            Ok(bytes_read) => Ok(bytes_read),
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => Ok(0),
            Err(e) => Err(Error::Io(e)),
        };

        // Reset read to non-blocking mode
        termios::configure_read(self.device.as_raw_fd(), 0, Duration::default())?;

        return_value
    }

    /// Discards all waiting incoming and outgoing data.
    pub fn flush(&self) -> Result<()> {
        termios::flush(self.device.as_raw_fd())
    }

    /// Blocks until all waiting outgoing data has been transmitted.
    pub fn drain(&self) -> Result<()> {
        termios::drain(self.device.as_raw_fd())
    }
}

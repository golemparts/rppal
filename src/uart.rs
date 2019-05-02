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

//! Interface for the UART peripherals and any USB to serial adapters.
//!
//! RPPAL controls the Raspberry Pi's UART peripherals through the `ttyAMA0`
//! (PL011) and `ttyS0` (mini UART) character devices. USB to serial adapters
//! are controlled using the `ttyUSBx` and `ttyACMx` character devices.
//!
//! ## UART peripherals
//!
//! The Raspberry Pi's BCM283x SoC features two UART peripherals.
//! `/dev/ttyAMA0` represents the PL011 UART, which offers a full set of
//! features. `/dev/ttyS0` represents an auxiliary peripheral that's referred
//! to as mini UART, with limited capabilities. More details on the differences
//! between the PL011 and mini UART can be found in the official Raspberry Pi
//! [documentation].
//!
//! On earlier Raspberry Pi models without Bluetooth, `/dev/ttyAMA0` is
//! configured as a Linux serial console. On more recent models with Bluetooth
//! (3A+, 3B, 3B+, Zero W), `/dev/ttyAMA0` is connected to the Bluetooth
//! module, and `/dev/ttyS0` is used as a serial console instead. Due to the
//! limitations of `/dev/ttyS0` and the requirement for a fixed core frequency,
//! in most cases you'll want to use `/dev/ttyAMA0` for serial communication.
//!
//! By default, TX (outgoing data) is tied to BCM GPIO 14 (physical pin 8) and
//! RX (incoming data) is tied to BCM GPIO 15 (physical pin 10). You can move
//! these lines to different GPIO pins using the `uart0` and `uart1` overlays,
//! but the alternative pin options aren't exposed through the GPIO header on
//! any of the current Raspberry Pi models. They are only available on the
//! Compute Module's SO-DIMM pads.
//!
//! ## Configure `/dev/ttyAMA0` for serial communication (recommended)
//!
//! Disable the Linux serial console by either deactivating it through
//! `sudo raspi-config`, or manually removing the parameter
//! `console=serial0,115200` from `/boot/cmdline.txt`.
//!
//! Remove any lines containing `enable_uart=0` or `enable_uart=1` from
//! `/boot/config.txt`.
//!
//! On Raspberry Pi models with a Bluetooth module, an extra step is required
//! to either disable Bluetooth or move it to `/dev/ttyS0`, so `/dev/ttyAMA0`
//! becomes available for serial communication.
//!
//! To disable Bluetooth, add `dtoverlay=pi3-disable-bt` to `/boot/config.txt`.
//! You'll also need to disable the service that initializes Bluetooth with
//! `sudo systemctl disable hciuart`.
//!
//! To move the Bluetooth module to `/dev/ttyS0`, instead of disabling it with
//! the above-mentioned steps, add `dtoverlay=pi3-miniuart-bt` and
//! `core_freq=250` to `/boot/config.txt`.
//!
//! Remember to reboot the Raspberry Pi after making any changes.
//!
//! ## Configure `/dev/ttyS0` for serial communication
//!
//! If you prefer to leave the Bluetooth module connected to `/dev/ttyAMA0`,
//! you can configure `/dev/ttyS0` for serial communication instead.
//!
//! Disable the Linux serial console by either deactivating it through
//! `sudo raspi-config`, or manually removing the parameter
//! `console=serial0,115200` from `/boot/cmdline.txt`.
//!
//! Add the line `enable_uart=1` to `/boot/config.txt` to enable serial
//! communication on `/dev/ttyS0`, which also sets a fixed core frequency.
//!
//! Remember to reboot the Raspberry Pi after making any changes.
//!
//! ## USB to serial adapters
//!
//! In addition to controlling the hardware UART peripherals, [`Uart`] can
//! also be used for USB to serial adapters. Depending on the type of
//! device, these can be accessed either through `/dev/ttyUSBx` or
//! `/dev/ttyACMx`, where `x` is an index starting at `0`. The numbering is
//! based on the order in which the devices are discovered by the kernel.
//!
//! When you have multiple USB to serial adapters connected at the same time,
//! you can uniquely identify a specific device by searching for the relevant
//! symlink in the `/dev/serial/by-id` directory, or by adding your own
//! `udev` rules.
//!
//! Support for automatic software (XON/XOFF) and hardware (RTS/CTS) flow
//! control for USB to serial adapters depends on the USB interface IC on the
//! device, and the relevant Linux driver. Some ICs use an older,
//! incompatible RTS/CTS implementation, sometimes referred to as legacy or
//! simplex mode, where RTS is used to indicate data is about to be
//! transmitted, rather than to request the external device to resume its
//! transmission.
//!
//! ## Hardware flow control
//!
//! The RTS/CTS hardware flow control implementation supported by [`Uart`]
//! and used by the Raspberry Pi's UART peripherals requires RTS on one
//! device to be connected to CTS on the other device. The RTS signal is
//! used to request the other device to pause or resume its transmission.
//!
//! Some devices use an older, incompatible RTS/CTS implementation, sometimes
//! referred to as legacy or simplex mode, where RTS is connected to RTS, and
//! CTS to CTS. The RTS signal is used to indicate data is about to be
//! transmitted. [`Uart`] is not compatible with this implementation.
//! Connecting the Raspberry Pi's RTS and CTS pins incorrectly could damage
//! the Pi or the external device.
//!
//! When [`Uart`] is controlling a UART peripheral, enabling hardware flow
//! control will also configure the RTS and CTS pins. On Raspberry Pi models
//! with a 40-pin GPIO header, RTS is tied to BCM GPIO 17 (physical pin 11)
//! and CTS is tied to BCM GPIO 16 (physical pin 36). RTS and CTS aren't
//! available on models with a 26-pin header, except for the Raspberry Pi B
//! Rev 2, which exposes RTS and CTS through its unpopulated P5 header with
//! RTS on BCM GPIO 31 (physical pin 6) and CTS on BCM GPIO 30 (physical pin
//! 5).
//!
//! The RTS and CTS pins are reset to their original state when [`Uart`] goes
//! out of scope. Note that `drop` methods aren't called when a process is
//! abnormally terminated, for instance when a user presses <kbd>Ctrl</kbd> +
//! <kbd>C</kbd> and the `SIGINT` signal isn't caught, which prevents [`Uart`]
//! from resetting the pins. You can catch those using crates such as
//! [`simple_signal`].
//!
//! ## Troubleshooting
//!
//! ### Permission denied
//!
//! If [`new`] or [`with_path`] returns an `io::ErrorKind::PermissionDenied`
//! error, make sure the file permissions for the specified device are correct,
//! and the current user is a member of the group that owns the device, which is
//! usually either `dialout` or `tty`.
//!
//! [documentation]: https://www.raspberrypi.org/documentation/configuration/uart.md
//! [`simple_signal`]: https://crates.io/crates/simple-signal
//! [`Uart`]: struct.Uart.html
//! [`new`]: struct.Uart.html#method.new
//! [`with_path`]: struct.Uart.html#method.with_path

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

use libc::{c_int, O_NOCTTY, O_NONBLOCK};
use libc::{TIOCM_CAR, TIOCM_CTS, TIOCM_DSR, TIOCM_DTR, TIOCM_RNG, TIOCM_RTS};

use crate::gpio::{self, Gpio, IoPin, Mode};
use crate::system::{self, DeviceInfo, Model};

#[cfg(feature = "hal")]
mod hal;
mod termios;

const GPIO_RTS: u8 = 17;
const GPIO_CTS: u8 = 16;

const GPIO_RTS_BREV2: u8 = 31;
const GPIO_CTS_BREV2: u8 = 30;

const GPIO_RTS_MODE_UART0: Mode = Mode::Alt3;
const GPIO_CTS_MODE_UART0: Mode = Mode::Alt3;

const GPIO_RTS_MODE_UART1: Mode = Mode::Alt5;
const GPIO_CTS_MODE_UART1: Mode = Mode::Alt5;

/// Errors that can occur when accessing the UART peripheral.
#[derive(Debug)]
pub enum Error {
    /// I/O error.
    Io(io::Error),
    /// GPIO error.
    Gpio(gpio::Error),
    /// Invalid or unsupported value.
    InvalidValue,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Error::Io(ref err) => write!(f, "I/O error: {}", err),
            Error::Gpio(ref err) => write!(f, "GPIO error: {}", err),
            Error::InvalidValue => write!(f, "Invalid or unsupported value"),
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

impl From<system::Error> for Error {
    fn from(_err: system::Error) -> Error {
        Error::Gpio(gpio::Error::UnknownModel)
    }
}

/// Result type returned from methods that can have `uart::Error`s.
pub type Result<T> = result::Result<T, Error>;

/// Parity bit modes.
///
/// The parity bit mode determines how the parity bit is calculated.
///
/// `None` omits the parity bit. `Even` and `Odd` count the total number of
/// 1-bits in the data bits. `Mark` and `Space` always set the parity
/// bit to `1` or `0` respectively.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Parity {
    /// No parity bit.
    None,
    /// Even parity.
    Even,
    /// Odd parity.
    Odd,
    /// Sets parity bit to `1`.
    Mark,
    /// Sets parity bit to `0`.
    Space,
}

impl fmt::Display for Parity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Parity::None => write!(f, "None"),
            Parity::Even => write!(f, "Even"),
            Parity::Odd => write!(f, "Odd"),
            Parity::Mark => write!(f, "Mark"),
            Parity::Space => write!(f, "Space"),
        }
    }
}

/// Parity check modes.
///
/// The parity check mode determines how parity errors are handled.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum ParityCheck {
    /// Ignores parity errors.
    None,
    /// Removes bytes with parity errors from the input queue.
    Strip,
    /// Replaces bytes with parity errors with a `0` byte.
    Replace,
    /// Marks bytes with parity errors with a preceding `255` and `0` byte.
    ///
    /// Actual `255` bytes are replaced with two `255` bytes to avoid confusion
    /// with parity errors.
    Mark,
}

impl fmt::Display for ParityCheck {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            ParityCheck::None => write!(f, "None"),
            ParityCheck::Strip => write!(f, "Strip"),
            ParityCheck::Replace => write!(f, "Replace"),
            ParityCheck::Mark => write!(f, "Mark"),
        }
    }
}

/// Queue types.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Queue {
    /// Input queue.
    Input,
    /// Output queue.
    Output,
    /// Both queues.
    Both,
}

impl fmt::Display for Queue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Queue::Input => write!(f, "Input"),
            Queue::Output => write!(f, "Output"),
            Queue::Both => write!(f, "Both"),
        }
    }
}

/// Control signal status.
pub struct Status {
    tiocm: c_int,
}

impl Status {
    /// Returns `true` if RTS is active.
    ///
    /// RTS (active low) is controlled by [`Uart`]. An active signal indicates
    /// [`Uart`] is ready to receive more data.
    ///
    /// [`Uart`]: struct.Uart.html
    pub fn rts(&self) -> bool {
        self.tiocm & TIOCM_RTS > 0
    }

    /// Returns `true` if CTS is active.
    ///
    /// CTS (active low) is controlled by the external device. An active signal
    /// indicates the external device is ready to receive more data.
    pub fn cts(&self) -> bool {
        self.tiocm & TIOCM_CTS > 0
    }

    /// Returns `true` if DTR is active.
    ///
    /// DTR (active low) is controlled by [`Uart`]. When communicating with a
    /// modem, an active signal is used to place or accept a call. An inactive
    /// signal causes the modem to hang up. Other devices may use DTR and DSR
    /// for flow control.
    ///
    /// DTR is not supported by the Raspberry Pi's UART peripherals,
    /// but may be available on some USB to serial adapters.
    ///
    /// [`Uart`]: struct.Uart.html
    pub fn dtr(&self) -> bool {
        self.tiocm & TIOCM_DTR > 0
    }

    /// Returns `true` if DSR is active.
    ///
    /// DSR (active low) is controlled by the external device. When
    /// communicating with a modem, an active signal indicates the modem is
    /// ready for data transmission. Other devices may use DTR and DSR for flow
    /// control.
    ///
    /// DSR is not supported by the Raspberry Pi's UART peripherals,
    /// but may be available on some USB to serial adapters.
    pub fn dsr(&self) -> bool {
        self.tiocm & TIOCM_DSR > 0
    }

    /// Returns `true` if DCD is active.
    ///
    /// DCD (active low) is controlled by the external device. When
    /// communicating with a modem, an active signal indicates a connection is
    /// established.
    ///
    /// DCD is not supported by the Raspberry Pi's UART peripherals,
    /// but may be available on some USB to serial adapters.
    pub fn dcd(&self) -> bool {
        self.tiocm & TIOCM_CAR > 0
    }

    /// Returns `true` if RI is active.
    ///
    /// RI (active low) is controlled by the external device. When
    /// communicating with a modem, an active signal indicates an incoming
    /// call.
    ///
    /// RI is not supported by the Raspberry Pi's UART peripherals,
    /// but may be available on some USB to serial adapters.
    pub fn ri(&self) -> bool {
        self.tiocm & TIOCM_RNG > 0
    }
}

impl fmt::Debug for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Status")
            .field("rts", &self.rts())
            .field("cts", &self.cts())
            .field("dtr", &self.dtr())
            .field("dsr", &self.dsr())
            .field("dcd", &self.dcd())
            .field("ri", &self.ri())
            .finish()
    }
}

#[derive(Debug)]
struct UartInner {
    device: File,
    fd: RawFd,
    rtscts_mode: Option<(Mode, Mode)>,
    rtscts_pins: Option<(IoPin, IoPin)>,
    blocking_read: bool,
    blocking_write: bool,
    baud_rate: u32,
    parity: Parity,
    parity_check: ParityCheck,
    data_bits: u8,
    stop_bits: u8,
    software_flow_control: bool,
    hardware_flow_control: bool,
}

/// Provides access to the Raspberry Pi's UART peripherals and any USB to
/// serial adapters.
///
/// The `embedded-hal` [`serial::Read`], [`serial::Write`] and [`blocking::serial::Write`] trait
/// implementations for `Uart` can be enabled by specifying the optional `hal` feature in the
/// dependency declaration for the `rppal` crate.
///
/// [`serial::Read`]: ../../embedded_hal/serial/trait.Read.html
/// [`serial::Write`]: ../../embedded_hal/serial/trait.Write.html
/// [`blocking::serial::Write`]: ../../embedded_hal/blocking/serial/trait.Write.html
#[derive(Debug)]
pub struct Uart {
    inner: UartInner,
}

impl Uart {
    /// Constructs a new `Uart`.
    ///
    /// `new` attempts to identify the UART peripheral tied to BCM GPIO 14 and
    /// 15, and then calls [`with_path`] with the appropriate device path.
    ///
    /// [`with_path`]: #method.with_path
    pub fn new(baud_rate: u32, parity: Parity, data_bits: u8, stop_bits: u8) -> Result<Uart> {
        Self::with_path("/dev/serial0", baud_rate, parity, data_bits, stop_bits)
    }

    /// Constructs a new `Uart` connected to the serial character device
    /// specified by `path`.
    ///
    /// `with_path` can be used to connect to either a UART peripheral or a USB
    /// to serial adapter.
    ///
    /// When a new `Uart` is constructed, the specified device is configured
    /// for non-canonical mode which processes input per character, ignores any
    /// special terminal input or output characters and disables local echo. DCD
    /// is ignored, all flow control is disabled, and the input and output queues
    /// are flushed.
    pub fn with_path<P: AsRef<Path>>(
        path: P,
        baud_rate: u32,
        parity: Parity,
        data_bits: u8,
        stop_bits: u8,
    ) -> Result<Uart> {
        // Follow symbolic links
        let path = fs::canonicalize(path)?;

        // Check if we're using /dev/ttyAMA0 or /dev/ttyS0 so we can set the
        // correct RTS/CTS pin modes when needed.
        let rtscts_mode = if let Some(path_str) = path.to_str() {
            match path_str {
                "/dev/ttyAMA0" => Some((GPIO_RTS_MODE_UART0, GPIO_CTS_MODE_UART0)),
                "/dev/ttyS0" => Some((GPIO_RTS_MODE_UART1, GPIO_CTS_MODE_UART1)),
                _ => None,
            }
        } else {
            None
        };

        let device = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(O_NOCTTY | O_NONBLOCK)
            .open(path)?;

        let fd = device.as_raw_fd();

        // Enables character input mode, disables echoing and any special
        // processing
        termios::set_raw_mode(fd)?;

        // Non-blocking reads
        termios::set_read_mode(fd, 0, Duration::default())?;

        // Ignore modem control lines (CLOCAL)
        termios::ignore_carrier_detect(fd)?;

        // Enable receiver (CREAD)
        termios::enable_read(fd)?;

        // Disable software flow control (XON/XOFF)
        termios::set_software_flow_control(fd, false, false)?;

        // Disable hardware flow control (RTS/CTS)
        termios::set_hardware_flow_control(fd, false)?;

        termios::set_line_speed(fd, baud_rate)?;
        termios::set_parity(fd, parity)?;
        termios::set_data_bits(fd, data_bits)?;
        termios::set_stop_bits(fd, stop_bits)?;

        // Pass through parity errors unfiltered
        termios::set_parity_check(fd, ParityCheck::None)?;

        // Flush the input and output queue
        termios::flush(fd, Queue::Both)?;

        Ok(Uart {
            inner: UartInner {
                device,
                fd,
                rtscts_mode,
                rtscts_pins: None,
                blocking_read: false,
                blocking_write: false,
                baud_rate,
                parity,
                parity_check: ParityCheck::None,
                data_bits,
                stop_bits,
                software_flow_control: false,
                hardware_flow_control: false,
            },
        })
    }

    /// Returns the line speed in baud (Bd).
    pub fn baud_rate(&self) -> u32 {
        self.inner.baud_rate
    }

    /// Sets the line speed in baud (Bd).
    ///
    /// On the Raspberry Pi, baud rate is equivalent to bit rate in bits per
    /// second (bit/s).
    ///
    /// Accepted values:
    /// `0`, `50`, `75`, `110`, `134`, `150`, `200`, `300`, `600`, `1_200`,
    /// `1_800`, `2_400`, `4_800`, `9_600`, `19_200`, `38_400`, `57_600`,
    /// `115_200`, `230_400`, `460_800`, `500_000`, `576_000`, `921_600`,
    /// `1_000_000`, `1_152_000`, `1_500_000`, `2_000_000`, `2_500_000`,
    /// `3_000_000`, `3_500_000`, `4_000_000`.
    ///
    /// Support for some values may be device-dependent.
    pub fn set_baud_rate(&mut self, baud_rate: u32) -> Result<()> {
        termios::set_line_speed(self.inner.fd, baud_rate)?;

        self.inner.baud_rate = baud_rate;

        Ok(())
    }

    /// Returns the parity bit mode.
    pub fn parity(&self) -> Parity {
        self.inner.parity
    }

    /// Sets the parity bit mode.
    ///
    /// The parity bit mode determines how the parity bit is calculated.
    ///
    /// Support for some modes may be device-dependent.
    pub fn set_parity(&mut self, parity: Parity) -> Result<()> {
        termios::set_parity(self.inner.fd, parity)?;

        self.inner.parity = parity;

        Ok(())
    }

    /// Returns the parity check mode for incoming data.
    pub fn parity_check(&self) -> ParityCheck {
        self.inner.parity_check
    }

    /// Configures parity checking for incoming data.
    ///
    /// The parity check mode determines how parity errors are handled.
    ///
    /// By default, `parity_check` is set to [`None`].
    ///
    /// Support for some modes may be device-dependent.
    ///
    /// [`None`]: enum.ParityCheck.html#variant.None
    pub fn set_parity_check(&mut self, parity_check: ParityCheck) -> Result<()> {
        termios::set_parity_check(self.inner.fd, parity_check)?;

        self.inner.parity_check = parity_check;

        Ok(())
    }

    /// Returns the number of data bits.
    pub fn data_bits(&self) -> u8 {
        self.inner.data_bits
    }

    /// Sets the number of data bits.
    ///
    /// Accepted values: `5`, `6`, `7`, `8`.
    ///
    /// Support for some values may be device-dependent.
    pub fn set_data_bits(&mut self, data_bits: u8) -> Result<()> {
        termios::set_data_bits(self.inner.fd, data_bits)?;

        self.inner.data_bits = data_bits;

        Ok(())
    }

    /// Returns the number of stop bits.
    pub fn stop_bits(&self) -> u8 {
        self.inner.stop_bits
    }

    /// Sets the number of stop bits.
    ///
    /// Accepted values: `1`, `2`.
    ///
    /// Support for some values may be device-dependent.
    pub fn set_stop_bits(&mut self, stop_bits: u8) -> Result<()> {
        termios::set_stop_bits(self.inner.fd, stop_bits)?;

        self.inner.stop_bits = stop_bits;

        Ok(())
    }

    /// Returns the status of the control signals.
    pub fn status(&self) -> Result<Status> {
        let tiocm = termios::status(self.inner.fd)?;

        Ok(Status { tiocm })
    }

    /// Sets DTR to active (`true`) or inactive (`false`).
    ///
    /// DTR is not supported by the Raspberry Pi's UART peripherals,
    /// but may be available on some USB to serial adapters.
    pub fn set_dtr(&mut self, dtr: bool) -> Result<()> {
        termios::set_dtr(self.inner.fd, dtr)
    }

    /// Sets RTS to active (`true`) or inactive (`false`).
    pub fn set_rts(&mut self, rts: bool) -> Result<()> {
        termios::set_rts(self.inner.fd, rts)
    }

    /// Returns `true` if XON/XOFF software flow control is enabled.
    pub fn software_flow_control(&self) -> bool {
        self.inner.software_flow_control
    }

    /// Enables or disables XON/XOFF software flow control.
    ///
    /// When software flow control is enabled, incoming XON (decimal 17) and
    /// XOFF (decimal 19) control characters are filtered from the input queue.
    /// When XOFF is received, the transmission of data in the output queue is
    /// paused until the external device sends XON. XOFF is automatically sent
    /// to the external device to prevent the input queue from overflowing.
    /// XON is sent when the input queue is ready for more data. You can also
    /// manually send these control characters by calling [`send_stop`] and
    /// [`send_start`].
    ///
    /// By default, software flow control is disabled.
    ///
    /// Support for XON/XOFF software flow control is
    /// device-dependent. You can manually implement XON/XOFF by disabling
    /// software flow control, parsing incoming XON/XOFF control characters
    /// received with [`read`], and sending XON/XOFF when needed using
    /// [`write`].
    ///
    /// [`send_start`]: #method.send_start
    /// [`send_stop`]: #method.send_stop
    /// [`read`]: #method.read
    /// [`write`]: #method.write
    pub fn set_software_flow_control(&mut self, software_flow_control: bool) -> Result<()> {
        termios::set_software_flow_control(
            self.inner.fd,
            software_flow_control,
            software_flow_control,
        )?;

        self.inner.software_flow_control = software_flow_control;

        Ok(())
    }

    /// Returns `true` if RTS/CTS hardware flow control is enabled.
    pub fn hardware_flow_control(&self) -> bool {
        self.inner.hardware_flow_control
    }

    /// Enables or disables RTS/CTS hardware flow control.
    ///
    /// When hardware flow control is enabled, the RTS line (active low) is
    /// automatically driven high to prevent the input queue from overflowing,
    /// and driven low when the input queue is ready for more data. When the
    /// CTS line (active low) is driven high by the external device, all data
    /// in the output queue is held until CTS is driven low. You can also
    /// manually change the active state of RTS by calling [`send_stop`] and
    /// [`send_start`].
    ///
    /// When `Uart` is controlling a UART peripheral, enabling hardware flow
    /// control will also configure the RTS and CTS pins.
    ///
    /// More information on hardware flow control can be found [here].
    ///
    /// By default, hardware flow control is disabled.
    ///
    /// Support for RTS/CTS hardware flow control is device-dependent. You can
    /// manually implement RTS/CTS using [`cts`], [`send_stop`] and
    /// [`send_start`], or by disabling hardware flow control and configuring
    /// an [`OutputPin`] for RTS and an [`InputPin`] for CTS.
    ///
    /// [here]: index.html#hardware-flow-control
    /// [`cts`]: struct.Status.html#method.cts
    /// [`send_start`]: #method.send_start
    /// [`send_stop`]: #method.send_stop
    /// [`OutputPin`]: ../gpio/struct.OutputPin.html
    /// [`InputPin`]: ../gpio/struct.InputPin.html
    pub fn set_hardware_flow_control(&mut self, hardware_flow_control: bool) -> Result<()> {
        if hardware_flow_control && self.inner.rtscts_pins.is_none() {
            // Configure and store RTS/CTS GPIO pins for UART0/UART1, so their
            // mode is automatically reset when Uart goes out of scope.
            if let Some((rts_mode, cts_mode)) = self.inner.rtscts_mode {
                let gpio = Gpio::new()?;

                let (gpio_rts, gpio_cts) = if DeviceInfo::new()?.model() == Model::RaspberryPiBRev2
                {
                    // The Pi B Rev 2 exposes RTS/CTS through its (unpopulated) P5 header
                    (GPIO_RTS_BREV2, GPIO_CTS_BREV2)
                } else {
                    // All other models with a 40-pin header use these GPIO pins
                    (GPIO_RTS, GPIO_CTS)
                };

                let pin_rts = gpio.get(gpio_rts)?.into_io(rts_mode);
                let pin_cts = gpio.get(gpio_cts)?.into_io(cts_mode);

                self.inner.rtscts_pins = Some((pin_rts, pin_cts));
            }
        } else if !hardware_flow_control {
            self.inner.rtscts_pins = None;
        }

        termios::set_hardware_flow_control(self.inner.fd, hardware_flow_control)?;

        self.inner.hardware_flow_control = hardware_flow_control;

        Ok(())
    }

    /// Requests the external device to pause its transmission using flow control.
    ///
    /// If software flow control is enabled, `send_stop`
    /// sends the XOFF control character.
    ///
    /// If hardware flow control is enabled, `send_stop` sets RTS to its
    /// inactive state.
    pub fn send_stop(&self) -> Result<()> {
        if self.inner.software_flow_control {
            termios::send_stop(self.inner.fd)?;
        }

        if self.inner.hardware_flow_control {
            termios::set_rts(self.inner.fd, false)?;
        }

        Ok(())
    }

    /// Requests the external device to resume its transmission using flow control.
    ///
    /// If software flow control is enabled, `send_start`
    /// sends the XON control character.
    ///
    /// If hardware flow control is enabled, `send_start` sets RTS to its
    /// active state.
    pub fn send_start(&self) -> Result<()> {
        if self.inner.software_flow_control {
            termios::send_start(self.inner.fd)?;
        }

        if self.inner.hardware_flow_control {
            termios::set_rts(self.inner.fd, true)?;
        }

        Ok(())
    }

    /// Returns `true` if [`read`] is configured to block when needed.
    ///
    /// [`read`]: #method.write
    pub fn is_read_blocking(&self) -> bool {
        self.inner.blocking_read
    }

    /// Returns `true` if [`write`] is configured to block when needed.
    ///
    /// [`write`]: #method.write
    pub fn is_write_blocking(&self) -> bool {
        self.inner.blocking_write
    }

    /// Sets the blocking mode for subsequent calls to [`read`].
    ///
    /// `min_length` indicates the minimum number of requested bytes. This
    /// value may differ from the actual buffer length. Maximum value: 255
    /// bytes.
    ///
    /// `timeout` indicates how long [`read`] blocks while waiting for
    /// incoming data. `timeout` uses a 0.1 second resolution. Maximum
    /// value: 25.5 seconds.
    ///
    /// [`read`] operates in one of four modes, depending on the specified
    /// `min_length` and `timeout` values:
    ///
    /// * **Non-blocking read** (`min_length` = 0, `timeout` = 0). [`read`]
    /// retrieves any available data and returns immediately.
    /// * **Blocking read** (`min_length` > 0, `timeout` = 0). [`read`] blocks
    /// until at least `min_length` bytes are available, or the provided buffer
    /// is full.
    /// * **Read with timeout** (`min_length` = 0, `timeout` > 0). [`read`]
    /// blocks until at least one byte is available, or the `timeout` duration
    /// elapses.
    /// * **Read with inter-byte timeout** (`min_length` > 0, `timeout` > 0).
    /// [`read`] blocks until at least `min_length` bytes are available, the
    /// provided buffer is full, or the `timeout` duration elapses
    /// after receiving one or more bytes. The timer is started after an
    /// initial byte becomes available, and is restarted after each additional
    /// byte. That means [`read`] will block indefinitely until at least one
    /// byte has been received.
    ///
    /// By default, [`read`] is configured as non-blocking.
    ///
    /// [`read`]: #method.read
    pub fn set_read_mode(&mut self, min_length: u8, timeout: Duration) -> Result<()> {
        termios::set_read_mode(self.inner.fd, min_length, timeout)?;

        self.inner.blocking_read = min_length > 0 || timeout.as_millis() > 0;

        // If both read() and write() are non-blocking, we can safely set
        // O_NONBLOCK once instead of toggling it for every write. We can't
        // leave it set when read() should block, because it ignores the
        // VMIN and VTIME settings.
        if self.inner.blocking_read || self.inner.blocking_write {
            unsafe {
                libc::fcntl(self.inner.fd, libc::F_SETFL, 0);
            }
        } else {
            unsafe {
                libc::fcntl(self.inner.fd, libc::F_SETFL, libc::O_NONBLOCK);
            }
        }

        Ok(())
    }

    /// Sets the blocking mode for subsequent calls to [`write`].
    ///
    /// [`write`] operates in one of two modes, depending on the specified
    /// `blocking` value:
    ///
    /// * **Non-blocking write**. [`write`] returns immediately after
    /// copying as much of the contents of the provided buffer to the output queue
    /// as it's able to fit.
    /// * **Blocking write**. [`write`] blocks until the entire contents of the provided buffer
    /// can be copied to the output queue. If flow control is enabled and the
    /// external device has sent a stop request, the transmission of any waiting data
    /// in the output queue is paused until a start request has been received.
    ///
    /// By default, [`write`] is configured as non-blocking.
    ///
    /// [`write`]: #method.write
    pub fn set_write_mode(&mut self, blocking: bool) -> Result<()> {
        self.inner.blocking_write = blocking;

        // If both read() and write() are non-blocking, we can safely set
        // O_NONBLOCK once instead of toggling it for every write. We can't
        // leave it set when read() should block, because it ignores the
        // VMIN and VTIME settings.
        if self.inner.blocking_read || self.inner.blocking_write {
            unsafe {
                libc::fcntl(self.inner.fd, libc::F_SETFL, 0);
            }
        } else {
            unsafe {
                libc::fcntl(self.inner.fd, libc::F_SETFL, libc::O_NONBLOCK);
            }
        }

        Ok(())
    }

    /// Returns the number of bytes waiting in the input queue.
    pub fn input_len(&self) -> Result<usize> {
        termios::input_len(self.inner.fd)
    }

    /// Returns the number of bytes waiting in the output queue.
    pub fn output_len(&self) -> Result<usize> {
        termios::output_len(self.inner.fd)
    }

    /// Receives incoming data from the external device and stores it in
    /// `buffer`.
    ///
    /// `read` operates in one of four (non)blocking modes, depending on the
    /// settings configured by [`set_read_mode`]. By default, `read` is configured
    /// as non-blocking.
    ///
    /// Returns how many bytes were read.
    ///
    /// [`set_read_mode`]: #method.set_read_mode
    pub fn read(&mut self, buffer: &mut [u8]) -> Result<usize> {
        self.inner.device.read(buffer).or_else(|e| {
            if e.kind() == io::ErrorKind::WouldBlock {
                Ok(0)
            } else {
                Err(Error::Io(e))
            }
        })
    }

    /// Sends the contents of `buffer` to the external device.
    ///
    /// `write` operates in either blocking or non-blocking mode, depending on the
    /// settings configured by [`set_write_mode`]. By default, `write` is configured
    /// as non-blocking.
    ///
    /// Returns how many bytes were written.
    ///
    /// [`set_write_mode`]: #method.set_write_mode
    pub fn write(&mut self, buffer: &[u8]) -> Result<usize> {
        // We only need to toggle O_NONBLOCK when read() is configured as
        // blocking. If read() is non-blocking, either with_path() or
        // set_read_mode() will have already enabled O_NONBLOCK.
        if self.inner.blocking_read && !self.inner.blocking_write {
            unsafe {
                libc::fcntl(self.inner.fd, libc::F_SETFL, libc::O_NONBLOCK);
            }
        }

        let result = self.inner.device.write(buffer).or_else(|e| {
            if e.kind() == io::ErrorKind::WouldBlock {
                Ok(0)
            } else {
                Err(Error::Io(e))
            }
        });

        if self.inner.blocking_read && !self.inner.blocking_write {
            unsafe {
                libc::fcntl(self.inner.fd, libc::F_SETFL, 0);
            }
        }

        result
    }

    /// Blocks until all data in the output queue has been transmitted.
    pub fn drain(&self) -> Result<()> {
        termios::drain(self.inner.fd)
    }

    /// Discards all data in the input and/or output queue.
    pub fn flush(&self, queue_type: Queue) -> Result<()> {
        termios::flush(self.inner.fd, queue_type)
    }
}

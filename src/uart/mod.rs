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

//! Interface for the UART peripheral.
//!
//!
//!
//! ## UART devices
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
//! ## USB devices (`/dev/ttyUSBx`, `/dev/ttyACMx`)
//!
//! ## Troubleshooting
//!
//! ### Permission Denied
//!
//!

use std::fs::{File, OpenOptions};
use std::io;
use std::os::unix::io::AsRawFd;
use std::result;

mod termios;

quick_error! {
/// Errors that can occur when accessing the UART peripheral.
    #[derive(Debug)]
    pub enum Error {
/// IO error.
        Io(err: io::Error) { description(err.description()) from() }
/// Invalid value.
        InvalidValue { description("invalid value") }
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
/// In addition to the built-in UARTs, `Uart` also supports USB to serial
/// converters and other USB devices with a UART interface. Depending on the type of device, these
/// can be accessed either through `/dev/ttyUSBx` or `/dev/ttyACMx`, where `x`
/// is an index starting at `0`. The numbering is based on the order
/// in which the devices are discovered by the kernel, so you'll need to find
/// a way to uniquely identify them when you have multiple connected
/// at the same time. For instance, you can find the assigned tty device name
/// based on the id in `/dev/serial/by-id`.
///
/// [here]: index.html
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Device {
    Uart0,
    Uart1,
    Acm(u8),
    Usb(u8),
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Parity {
    None,
    Even,
    Odd,
    Mark,
    Space,
}

pub struct Uart {
    device: File,
}

impl Uart {
    /// Constructs a new `Uart`.
    pub fn new(
        device: Device,
        speed: u32,
        parity: Parity,
        data_bits: u8,
        stop_bits: u8,
    ) -> Result<Uart> {
        let device = OpenOptions::new()
            .read(true)
            .write(true)
            .open(match device {
                Device::Uart0 => "/dev/ttyAMA0".to_owned(),
                Device::Uart1 => "/dev/ttyS0".to_owned(),
                Device::Acm(idx) => format!("/dev/ttyACM{}", idx),
                Device::Usb(idx) => format!("/dev/ttyUSB{}", idx),
            })?;

        // TODO: Configure UART for raw mode?

        Ok(Uart { device })
    }

    /// Gets the line speed in baud (Bd).
    pub fn line_speed(&self) -> Result<u32> {
        unsafe { Ok(termios::speed(self.device.as_raw_fd())?) }
    }

    /// Sets the line speed in baud (Bd).
    pub fn set_line_speed(&self, speed: u32) -> Result<()> {
        unsafe {
            termios::set_speed(self.device.as_raw_fd(), speed)?;
        }

        Ok(())
    }

    /// Gets the parity.
    pub fn parity(&self) -> Result<Parity> {
        unsafe { Ok(termios::parity(self.device.as_raw_fd())?) }
    }

    /// Sets the parity.
    pub fn set_parity(&self, parity: Parity) -> Result<()> {
        unsafe {
            termios::set_parity(self.device.as_raw_fd(), parity)?;
        }

        Ok(())
    }

    /// Gets the data bits.
    pub fn data_bits(&self) -> Result<u8> {
        unsafe { Ok(termios::data_bits(self.device.as_raw_fd())?) }
    }

    /// Sets the data bits.
    ///
    /// Valid values: 5, 6, 7, 8
    pub fn set_data_bits(&self, data_bits: u8) -> Result<()> {
        unsafe {
            termios::set_data_bits(self.device.as_raw_fd(), data_bits)?;
        }

        Ok(())
    }

    /// Gets the stop bits.
    pub fn stop_bits(&self) -> Result<u8> {
        unsafe { Ok(termios::stop_bits(self.device.as_raw_fd())?) }
    }

    /// Sets the stop bits.
    ///
    /// Valid values: 1, 2
    pub fn set_stop_bits(&self, stop_bits: u8) -> Result<()> {
        unsafe {
            termios::set_stop_bits(self.device.as_raw_fd(), stop_bits)?;
        }

        Ok(())
    }

    /// Returns the status of the RTS/CTS hardware flow control setting.
    pub fn hardware_flow_control(&self) {
        unimplemented!()
    }

    /// Enables or disabled RTS/CTS hardware flow control.
    pub fn set_hardware_flow_control(&self) {
        unimplemented!()
    }

    pub fn read(&self, buffer: &mut [u8]) -> Result<()> {
        unimplemented!()
    }

    pub fn write(&self, buffer: &[u8]) -> Result<()> {
        unimplemented!()
    }

    pub fn flush(&self) {
        unimplemented!()
    }

    pub fn drain(&self) {
        unimplemented!()
    }
}

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

//! Interface for the I2C peripherals.
//!
//! More information can be found in NXP's [UM10204] datasheet/user manual.
//!
//! ## I2C buses
//!
//! i2c_arm
//! i2c_vc
//! i2c-gpio
//!
//! ## Clock speed
//!
//! The Broadcom Serial Controller (BSC), responsible for the I2C
//! interface, supports data transfer rates up to 400kbit/s (Fast-mode).
//!
//! By default, the I2C bus speed is set to 100kbit/s (Standard-mode).
//!
//! i2c_arm_baudrate
//! i2c_vc_baudrate
//!
//!
//! ## Troubleshooting
//!
//! User must be in the `i2c` group, or have superuser privileges.
//!
//!
//! [UM10204]: https://www.nxp.com/docs/en/user-guide/UM10204.pdf

use std::fmt;
use std::fs::{File, OpenOptions};
use std::io;
use std::io::{Read, Write};
use std::marker::PhantomData;
use std::os::unix::io::AsRawFd;
use std::result;

use ioctl::i2c as ioctl;
use system;
use system::{DeviceInfo, Model};

quick_error! {
/// Errors that can occur when accessing the I2C peripherals.
    #[derive(Debug)]
    pub enum Error {
/// IO error.
        Io(err: io::Error) { description(err.description()) from() }
/// Invalid slave address.
///
/// I2C supports 7-bit and 10-bit addresses. Several 7-bit addresses
/// are reserved, and can't be used as slave addresses. A list of
/// those reserved addresses can be found [here].
///
/// [here]: https://www.i2c-bus.org/addressing/
        InvalidSlaveAddress(slave_address: u16) { description("invalid slave address") }
/// Unknown SoC.
///
/// Based on the output of `/proc/cpuinfo`, it wasn't possible to identify the Raspberry Pi's SoC.
        UnknownSoC { description("unknown SoC") }
    }
}

impl From<system::Error> for Error {
    fn from(_err: system::Error) -> Error {
        Error::UnknownSoC
    }
}

/// Result type returned from methods that can have `i2c::Error`s.
pub type Result<T> = result::Result<T, Error>;

pub struct I2c {
    bus: u8,
    i2cdev: File,
    // The not_sync field is a workaround to force !Sync. I2c isn't safe for
    // Sync because of ioctl() and the underlying drivers. This avoids needing
    // #![feature(optin_builtin_traits)] to manually add impl !Sync for I2c.
    not_sync: PhantomData<*const ()>,
}

impl I2c {
    /// Creates a new instance of `I2c`.
    ///
    /// `new` tries to identify which I2C bus is bound to physical pins 3 (SDA)
    /// and 5 (SCL) based on the Raspberry Pi model. For the early model B Rev 1,
    /// it will open bus 0. For every other model, it will use bus 1.
    ///
    /// More information on configuring the I2C buses, including bus speed, can
    /// be found [here].
    ///
    /// [here]: index.html
    pub fn new() -> Result<I2c> {
        match DeviceInfo::new()?.model() {
            Model::RaspberryPiBRev1 => I2c::with_bus(0),
            _ => I2c::with_bus(1),
        }
    }

    /// Creates a new instance of `I2c` using the specified bus.
    ///
    /// `bus` indicates the selected I2C bus. You'll typically want to select the
    /// bus that's bound to physical pins 3 (SDA) and 5 (SCL). On the Raspberry
    /// Pi Model B Rev 1, those pins are tied to bus 0. On every other Raspberry
    /// Pi model, they're tied to bus 1.
    ///
    /// More information on configuring the I2C buses, including bus speed, can
    /// be found [here].
    ///
    /// [here]: index.html
    pub fn with_bus(bus: u8) -> Result<I2c> {
        // bus is a u8, because any bus ID could potentially
        // be configured for bit banging I2C using i2c-gpio.
        let i2cdev = OpenOptions::new()
            .read(true)
            .write(true)
            .open(format!("/dev/i2c-{}", bus))?;

        Ok(I2c {
            bus,
            i2cdev,
            not_sync: PhantomData,
        })
    }

    /// Returns the bus speed in bits per second (bit/s).
    pub fn speed(&self) -> Result<u32> {
        let mut buffer = [0u8; 4];

        File::open(format!(
            "/sys/class/i2c-adapter/i2c-{}/of_node/clock-frequency",
            self.bus
        ))?.read_exact(&mut buffer)?;

        Ok(u32::from(buffer[3])
            | (u32::from(buffer[2]) << 8)
            | (u32::from(buffer[1]) << 16)
            | (u32::from(buffer[0]) << 24))
    }

    /// Sets a 7-bit or 10-bit slave address.
    ///
    /// `slave_address` refers to the slave device you're communicating with.
    /// The specified address shouldn't include the additional R/W bit.
    pub fn set_slave_address(&mut self, slave_address: u16) -> Result<()> {
        // Filter out reserved addresses
        if (slave_address < 8) || ((slave_address >> 3) == 0b1111) {
            return Err(Error::InvalidSlaveAddress(slave_address));
        }

        unsafe {
            ioctl::set_slave_address(self.i2cdev.as_raw_fd(), i32::from(slave_address))?;
        }

        Ok(())
    }

    /// Receives incoming data from the slave device and writes it to `buffer`.
    ///
    /// The I2C protocol doesn't indicate how much incoming data is waiting,
    /// so the maximum number of bytes read depends on the length of `buffer`.
    ///
    /// Returns how many bytes were read.
    pub fn read(&mut self, buffer: &mut [u8]) -> Result<usize> {
        Ok(self.i2cdev.read(buffer)?)
    }

    /// Sends the outgoing data contained in `buffer` to the slave device.
    ///
    /// Returns how many bytes were written.
    pub fn write(&mut self, buffer: &[u8]) -> Result<usize> {
        Ok(self.i2cdev.write(buffer)?)
    }
}

// Send is safe for I2c, but we're marked !Send because of the dummy pointer that's
// needed to force !Sync.
unsafe impl Send for I2c {}

impl fmt::Debug for I2c {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("I2c")
            .field("bus", &self.bus)
            .field("i2cdev", &self.i2cdev)
            .finish()
    }
}

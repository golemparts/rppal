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
//! The Broadcom Serial Controller (BSC) peripheral controls a proprietary bus
//! compliant with the I2C bus/interface. RPPAL communicates with the BSC
//! using the i2cdev device interface.
//!
//! ## I2C buses
//!
//! The Raspberry Pi's BCM283x SoC offers three I2C buses, however only one
//! of those should be used for slave devices you want to communicate with.
//! The other two buses are used internally as an HDMI interface, and for
//! HAT identification.
//!
//! The I2C bus connected to physical pins 3 (SDA) and 5 (SCL) is disabled by
//! default. You can enable it by running `sudo raspi-config`, or by manually
//! adding `dtparam=i2c_arm=on` to `/boot/config.txt`. Remember to reboot
//! the Raspberry Pi afterwards.
//!
//! In addition to the hardware I2C buses, it's possible to configure a
//! bit-banged I2C bus in software on any available GPIO pins through the `i2c-gpio`
//! device tree overlay. More details on enabling and configuring `i2c-gpio`
//! can be found in `/boot/overlays/README`.
//!
//! ## Transmission speed
//!
//! The Raspberry Pi's BCM283x SoC supports I2C data transfer rates up to
//! 400 kbit/s (Fast-mode).
//!
//! By default, the I2C bus clock speed is set to 100 kHz. Transferring
//! 1 bit takes 1 clock cycle. You can change the
//! transfer rate by adding `dtparam=i2c_arm_baudrate=X` to
//! `/boot/config.txt`, where `X` should be replaced with the
//! clock frequency in herz (Hz). Remember to reboot
//! the Raspberry Pi afterwards.
//!
//! ## Troubleshooting
//!
//! ### Permission Denied
//!
//! If constructing a new `I2c` instance returns a Permission Denied
//! IO error, make sure the file permissions for `/dev/i2c-1` or `/dev/i2c-0`
//! are correct, and the current user is a member of the `i2c` group.

#![allow(dead_code)]

use std::fs::{File, OpenOptions};
use std::io;
use std::io::{Read, Write};
use std::marker::PhantomData;
use std::os::unix::io::AsRawFd;
use std::result;

use libc::c_ulong;

use system;
use system::{DeviceInfo, Model};

mod ioctl;

pub use self::ioctl::Capabilities;

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
/// [here]: https://en.wikipedia.org/wiki/I%C2%B2C#Reserved_addresses_in_7-bit_address_space
        InvalidSlaveAddress(slave_address: u16) { description("invalid slave address") }
/// I2C/SMBus feature not supported.
///
/// The underlying drivers don't support the selected I2C feature or SMBus protocol.
        FeatureNotSupported { description("I2C/SMBus feature not supported") }
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

/// Provides access to the Raspberry Pi's I2C peripherals.
///
/// Before using `I2c`, make sure your Raspberry Pi has the necessary I2C buses
/// enabled. More information can be found [here].
///
/// Besides basic I2C communication through buffer reads and writes, `I2c` can
/// also be used with devices that require SMBus (System Management Bus). SMBus
/// is based on I2C, and defines more structured message transactions
/// through its various protocols. More details can be found in the latest SMBus
/// [specification].
///
/// [here]: index.html#i2c-buses
/// [specification]: http://smbus.org/specs/SMBus_3_1_20180319.pdf
#[derive(Debug)]
pub struct I2c {
    bus: u8,
    funcs: Capabilities,
    i2cdev: File,
    addr_10bit: bool,
    address: u16,
    // The not_sync field is a workaround to force !Sync. I2c isn't safe for
    // Sync because of ioctl() and the underlying drivers. This avoids needing
    // #![feature(optin_builtin_traits)] to manually add impl !Sync for I2c.
    not_sync: PhantomData<*const ()>,
}

impl I2c {
    /// Constructs a new `I2c`.
    ///
    /// `new` tries to identify which I2C bus is bound to physical pins 3 (SDA)
    /// and 5 (SCL) based on the Raspberry Pi model. For the early model B Rev 1,
    /// bus 0 is selected. For every other model, bus 1 is used.
    ///
    /// More information on configuring the I2C buses can be found [here].
    ///
    /// [here]: index.html#i2c-buses
    pub fn new() -> Result<I2c> {
        match DeviceInfo::new()?.model() {
            Model::RaspberryPiBRev1 => I2c::with_bus(0),
            _ => I2c::with_bus(1),
        }
    }

    /// Constructs a new `I2c` using the specified bus.
    ///
    /// `bus` indicates the selected I2C bus. You'll typically want to select the
    /// bus that's bound to physical pins 3 (SDA) and 5 (SCL). On the Raspberry
    /// Pi Model B Rev 1, those pins are tied to bus 0. On every other Raspberry
    /// Pi model, they're connected to bus 1.
    ///
    /// More information on configuring the I2C buses can be found [here].
    ///
    /// [here]: index.html#i2c-buses
    pub fn with_bus(bus: u8) -> Result<I2c> {
        // bus is a u8, because any 8-bit bus ID could potentially
        // be configured for bit banging I2C using i2c-gpio.
        let i2cdev = OpenOptions::new()
            .read(true)
            .write(true)
            .open(format!("/dev/i2c-{}", bus))?;

        let capabilities = unsafe { ioctl::funcs(i2cdev.as_raw_fd())? };

        // Disable 10-bit addressing if it's supported
        if capabilities.addr_10bit() {
            unsafe {
                ioctl::set_addr_10bit(i2cdev.as_raw_fd(), 0)?;
            }
        }

        // Disable PEC if it's supported
        if capabilities.smbus_pec() {
            unsafe {
                ioctl::set_pec(i2cdev.as_raw_fd(), 0)?;
            }
        }

        Ok(I2c {
            bus,
            funcs: capabilities,
            i2cdev,
            addr_10bit: false,
            address: 0,
            not_sync: PhantomData,
        })
    }

    /// Returns information on the functionality supported by the underlying drivers.
    ///
    /// The returned [`Capabilities`] instance lists the available
    /// I2C and SMBus features.
    ///
    /// [`Capabilities`]: struct.Capabilities.html
    pub fn capabilities(&self) -> Capabilities {
        self.funcs
    }

    /// Returns the I2C bus ID.
    pub fn bus(&self) -> u8 {
        self.bus
    }

    /// Returns the clock frequency in herz (Hz).
    pub fn clock_speed(&self) -> Result<u32> {
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
    /// The specified address shouldn't include the R/W bit.
    ///
    /// By default, 10-bit addressing is disabled, which means
    /// `set_slave_address` only accepts 7-bit addresses. 10-bit addressing
    /// can be enabled with [`set_addr_10bit`]. Note that setting a 7-bit
    /// address when 10-bit addressing is enabled won't correctly target a
    /// slave device that doesn't support 10-bit addresses.
    ///
    /// [`set_addr_10bit`]: #method.set_addr_10bit
    pub fn set_slave_address(&mut self, slave_address: u16) -> Result<()> {
        // linux/Documentation/i2c/ten-bit-addresses mentions adding
        // an 0xa000 offset to 10-bit addresses to prevent overlap with
        // 7-bit addresses. However, i2c-dev.c doesn't seem to have
        // that implemented and returns EINVALID for anything > 0x03FF.
        // TODO: Try 10-bit addresses both with and without the offset to make sure we're not missing something obvious.

        // Filter out reserved, invalid and unsupported addresses
        if (!self.addr_10bit
            && (slave_address < 8 || (slave_address >> 3) == 0b1111 || slave_address > 0x7F))
            || (self.addr_10bit && slave_address > 0x03FF)
        {
            return Err(Error::InvalidSlaveAddress(slave_address));
        }

        unsafe {
            ioctl::set_slave_address(self.i2cdev.as_raw_fd(), c_ulong::from(slave_address))?;
        }

        self.address = slave_address;

        Ok(())
    }

    fn set_timeout(&mut self, timeout: u32) -> Result<()> {
        // Set to private. Doesn't seem to work as expected.
        unsafe {
            ioctl::set_timeout(self.i2cdev.as_raw_fd(), timeout as c_ulong)?;
        }

        Ok(())
    }

    fn set_retries(&mut self, retries: u32) -> Result<()> {
        // Set to private. While i2cdev implements retries, the underlying drivers don't.
        unsafe {
            ioctl::set_retries(self.i2cdev.as_raw_fd(), retries as c_ulong)?;
        }

        Ok(())
    }

    /// Enables or disables 10-bit addressing.
    ///
    /// 10-bit addressing currently isn't supported on the Raspberry Pi, and returns
    /// an [`Error::FeatureNotSupported`] error unless underlying driver support is
    /// detected.
    ///
    /// By default, `addr_10bit` is set to `false`.
    pub fn set_addr_10bit(&mut self, addr_10bit: bool) -> Result<()> {
        if !self.capabilities().addr_10bit() {
            return Err(Error::FeatureNotSupported);
        }

        unsafe {
            ioctl::set_addr_10bit(self.i2cdev.as_raw_fd(), addr_10bit as c_ulong)?;
        }

        self.addr_10bit = addr_10bit;

        Ok(())
    }

    /// Receives incoming data from the slave device and writes it to `buffer`.
    ///
    /// `read` reads as many bytes as can fit in `buffer`.
    ///
    /// Sequence: START → Address + Read Bit → Incoming Bytes → STOP
    ///
    /// Returns how many bytes were read.
    pub fn read(&mut self, buffer: &mut [u8]) -> Result<usize> {
        // TODO: Is there a maximum buffer length?
        Ok(self.i2cdev.read(buffer)?)
    }

    /// Sends the outgoing data contained in `buffer` to the slave device.
    ///
    /// Sequence: START → Address + Write Bit → Outgoing Bytes → STOP
    ///
    /// Returns how many bytes were written.
    pub fn write(&mut self, buffer: &[u8]) -> Result<usize> {
        // TODO: Is there a maximum buffer length?
        Ok(self.i2cdev.write(buffer)?)
    }

    /// Sends the outgoing data contained in `write_buffer`, to the slave device, and
    /// then fills `read_buffer` with incoming data.
    ///
    /// Compared to calling [`write`] and [`read`], `write_read` doesn't issue a STOP
    /// condition in between the write and read operation. A repeated START is sent
    /// instead.
    ///
    /// `write_read` reads as many bytes as can fit in `read_buffer`. The maximum
    /// number of bytes in either `write_buffer` or `read_buffer` can't exceed 8192.
    ///
    /// Sequence: START → Address + Write Bit → Outgoing Bytes → Repeated START →
    /// Address + Read Bit → Incoming Bytes → STOP
    ///
    /// [`write`]: #method.write
    /// [`read`]: #method.read
    pub fn write_read(&mut self, write_buffer: &[u8], read_buffer: &mut [u8]) -> Result<()> {
        unsafe {
            ioctl::i2c_write_read(
                self.i2cdev.as_raw_fd(),
                self.address,
                write_buffer,
                read_buffer,
            )?;
        }

        Ok(())
    }

    /// Sends an 8-bit `command`, and then fills a multi-byte `buffer` with
    /// incoming data.
    ///
    /// `block_read` can read a maximum of 32 bytes. Any data that doesn't fit
    /// in `buffer` is discarded.
    ///
    /// Although `block_read` isn't part of the SMBus protocol, it uses the
    /// SMBus functionality to offer this commonly used I2C transaction format.
    /// The difference between `block_read` and [`smbus_block_read`] is that the
    /// latter also expects a byte count from the slave device.
    ///
    /// Sequence: START → Address + Write Bit → Command → Repeated START
    /// → Address + Read Bit → Incoming Bytes → STOP
    ///
    /// [`smbus_block_read`]: #method.smbus_block_read
    pub fn block_read(&self, command: u8, buffer: &mut [u8]) -> Result<()> {
        unsafe {
            ioctl::i2c_block_read(self.i2cdev.as_raw_fd(), command, buffer)?;
        }

        Ok(())
    }

    /// Sends an 8-bit `command` followed by a multi-byte `buffer`.
    ///
    /// `block_write` can write a maximum of 32 bytes. Any additional data contained
    /// in `buffer` is ignored.
    ///
    /// Although `block_write` isn't part of the SMBus protocol, it uses the
    /// SMBus functionality to offer this commonly used I2C transaction format. The
    /// difference between `block_write` and [`smbus_block_write`] is that the latter
    /// also sends a byte count to the slave device.
    ///
    /// Sequence: START → Address + Write Bit → Command → Outgoing Bytes → STOP
    ///
    /// [`smbus_block_write`]: #method.smbus_block_write
    pub fn block_write(&self, command: u8, buffer: &[u8]) -> Result<()> {
        unsafe {
            ioctl::i2c_block_write(self.i2cdev.as_raw_fd(), command, buffer)?;
        }

        Ok(())
    }

    // Note: smbus_read/write_32/64 could theoretically be emulated using block_read/write
    // provided the PEC value is calculated in software

    /// Sends a 1-bit `command` in place of the R/W bit.
    ///
    /// Sequence: START → Address + Command Bit → STOP
    pub fn smbus_quick_command(&self, command: bool) -> Result<()> {
        unsafe {
            ioctl::smbus_quick_command(self.i2cdev.as_raw_fd(), command)?;
        }

        Ok(())
    }

    /// Receives an 8-bit value.
    ///
    /// Sequence: START → Address + Read Bit → Incoming Byte → STOP
    pub fn smbus_receive_byte(&self) -> Result<u8> {
        unsafe { Ok(ioctl::smbus_receive_byte(self.i2cdev.as_raw_fd())?) }
    }

    /// Sends an 8-bit `value`.
    ///
    /// Sequence: START → Address + Write Bit → Outgoing Byte → STOP
    pub fn smbus_send_byte(&self, value: u8) -> Result<()> {
        unsafe {
            ioctl::smbus_send_byte(self.i2cdev.as_raw_fd(), value)?;
        }

        Ok(())
    }

    /// Sends an 8-bit `command`, and receives an 8-bit value.
    ///
    /// Sequence: START → Address + Write Bit → Command → Repeated START
    /// → Address + Read Bit → Incoming Byte → STOP
    pub fn smbus_read_byte(&self, command: u8) -> Result<u8> {
        unsafe { Ok(ioctl::smbus_read_byte(self.i2cdev.as_raw_fd(), command)?) }
    }

    /// Sends an 8-bit `command` and an 8-bit `value`.
    ///
    /// Sequence: START → Address + Write Bit → Command → Outgoing Byte → STOP
    pub fn smbus_write_byte(&self, command: u8, value: u8) -> Result<()> {
        unsafe {
            ioctl::smbus_write_byte(self.i2cdev.as_raw_fd(), command, value)?;
        }

        Ok(())
    }

    /// Sends an 8-bit `command`, and receives a 16-bit value.
    ///
    /// Based on the SMBus protocol definition, the first byte received is
    /// stored as the low byte of the 16-bit value, and the second byte as
    /// the high byte. Some devices may require you to swap these bytes. In those
    /// cases you can use the convenience method [`smbus_read_word_swapped`] instead.
    ///
    /// Sequence: START → Address + Write Bit → Command → Repeated START
    /// → Address + Read Bit → Incoming Byte Low → Incoming Byte High → STOP
    ///
    /// [`smbus_read_word_swapped`]: #method.smbus_read_word_swapped
    pub fn smbus_read_word(&self, command: u8) -> Result<u16> {
        unsafe { Ok(ioctl::smbus_read_word(self.i2cdev.as_raw_fd(), command)?) }
    }

    /// Sends an 8-bit `command`, and receives a 16-bit `value` in a non-standard swapped byte order.
    ///
    /// `smbus_read_word_swapped` is a convenience method that works similarly to [`smbus_read_word`],
    /// but reverses the byte order of the incoming 16-bit value. The high byte is received first,
    /// and the low byte second.
    ///
    /// [`smbus_read_word`]: #method.smbus_read_word
    pub fn smbus_read_word_swapped(&self, command: u8) -> Result<u16> {
        let value = unsafe { ioctl::smbus_read_word(self.i2cdev.as_raw_fd(), command)? };

        Ok(((value & 0xFF00) >> 8) | ((value & 0xFF) << 8))
    }

    /// Sends an 8-bit `command` and a 16-bit `value`.
    ///
    /// Based on the SMBus protocol definition, the first byte sent is the low byte
    /// of the 16-bit value, and the second byte is the high byte. Some devices may
    /// require you to swap these bytes. In those cases you can use the convenience method
    /// [`smbus_write_word_swapped`] instead.
    ///
    /// Sequence: START → Address + Write Bit → Command → Outgoing Byte Low → Outgoing Byte High → STOP
    ///
    /// [`smbus_write_word_swapped`]: #method.smbus_write_word_swapped
    pub fn smbus_write_word(&self, command: u8, value: u16) -> Result<()> {
        unsafe {
            ioctl::smbus_write_word(self.i2cdev.as_raw_fd(), command, value)?;
        }

        Ok(())
    }

    /// Sends an 8-bit `command` and a 16-bit `value` in a non-standard swapped byte order.
    ///
    /// `smbus_write_word_swapped` is a convenience method that works similarly to [`smbus_write_word`], but reverses the byte
    /// order of the outgoing 16-bit value. The high byte is sent first, and the low byte second.
    ///
    /// [`smbus_write_word`]: #method.smbus_write_word
    pub fn smbus_write_word_swapped(&self, command: u8, value: u16) -> Result<()> {
        unsafe {
            ioctl::smbus_write_word(
                self.i2cdev.as_raw_fd(),
                command,
                ((value & 0xFF00) >> 8) | ((value & 0xFF) << 8),
            )?;
        }

        Ok(())
    }

    /// Sends an 8-bit `command` and a 16-bit `value`, and then receives a 16-bit value in response.
    ///
    /// Based on the SMBus protocol definition, for both the outgoing and incoming 16-bit value,
    /// the first byte transferred is the low byte of the 16-bit value, and the second byte is the
    /// high byte. Some devices may require you to swap these bytes. In those cases you can use the
    /// convenience method [`smbus_process_call_swapped`] instead.
    ///
    /// Sequence: START → Address + Write Bit → Command → Outgoing Byte Low →
    /// Outgoing Byte High → Repeated START → Address + Read Bit → Incoming Byte Low →
    /// Incoming Byte High → STOP
    ///
    /// [`smbus_process_call_swapped`]: #method.smbus_process_call_swapped
    pub fn smbus_process_call(&self, command: u8, value: u16) -> Result<u16> {
        unsafe {
            Ok(ioctl::smbus_process_call(
                self.i2cdev.as_raw_fd(),
                command,
                value,
            )?)
        }
    }

    /// Sends an 8-bit `command` and a 16-bit `value`, and then receives a 16-bit value in response, in
    /// a non-standard byte order.
    ///
    /// `smbus_process_call_swapped` is a convenience method that works similarly to [`smbus_process_call`],
    /// but reverses the byte order of the outgoing and incoming 16-bit value. The high byte is transferred
    /// first, and the low byte second.
    ///
    /// [`smbus_process_call`]: #method.smbus_process_call
    pub fn smbus_process_call_swapped(&self, command: u8, value: u16) -> Result<u16> {
        let response = unsafe {
            ioctl::smbus_process_call(
                self.i2cdev.as_raw_fd(),
                command,
                ((value & 0xFF00) >> 8) | ((value & 0xFF) << 8),
            )?
        };

        Ok(((response & 0xFF00) >> 8) | ((response & 0xFF) << 8))
    }

    /// Sends an 8-bit 'command', and then receives an 8-bit byte count along with a
    /// multi-byte `buffer`.
    ///
    /// `smbus_block_read` currently isn't supported on the Raspberry Pi, and returns
    /// an [`Error::FeatureNotSupported`] error unless underlying driver support is
    /// detected. You might be able to emulate the `smbus_block_read` functionality
    /// with either [`block_read`] or [`read`] if the length of the expected incoming
    /// data is known beforehand, or if the slave device allows the master to read
    /// more data than it needs to send.
    ///
    /// `smbus_block_read` can read a maximum of 32 bytes. Any data that doesn't fit
    /// in `buffer` is discarded.
    ///
    /// Sequence: START → Address + Write Bit → Command → Repeated START →
    /// Address + Read Bit → Incoming Byte Count → Incoming Bytes → STOP
    ///
    /// Returns how many bytes were read.
    ///
    /// [`Error::FeatureNotSupported`]: enum.Error.html#variant.FeatureNotSupported
    /// [`block_read`]: #method.block_read
    /// [`read`]: #method.read
    pub fn smbus_block_read(&self, command: u8, buffer: &mut [u8]) -> Result<usize> {
        if !self.capabilities().smbus_block_read() {
            return Err(Error::FeatureNotSupported);
        }

        unsafe {
            Ok(ioctl::smbus_block_read(
                self.i2cdev.as_raw_fd(),
                command,
                buffer,
            )?)
        }
    }

    /// Sends an 8-bit `command` and an 8-bit byte count along with a multi-byte `buffer`.
    ///
    /// `smbus_block_write` can write a maximum of 32 bytes. Any additional data contained
    /// in `buffer` is ignored.
    ///
    /// Sequence: START → Address + Write Bit → Command → Outgoing Byte Count
    /// → Outgoing Bytes → STOP
    pub fn smbus_block_write(&self, command: u8, buffer: &[u8]) -> Result<()> {
        unsafe {
            ioctl::smbus_block_write(self.i2cdev.as_raw_fd(), command, buffer)?;
        }

        Ok(())
    }

    /// Enables or disables SMBus Packet Error Checking.
    ///
    /// Packet Error Checking inserts a CRC-8 Packet Error Code (PEC) byte before each STOP
    /// condition for all SMBus protocols, except Quick Command and Host Notify.
    ///
    /// The PEC is calculated on all message bytes except the START, STOP, ACK and NACK bits.
    ///
    /// By default, `pec` is set to `false`.
    pub fn set_smbus_pec(&self, pec: bool) -> Result<()> {
        unsafe {
            ioctl::set_pec(self.i2cdev.as_raw_fd(), pec as c_ulong)?;
        }

        Ok(())
    }
}

// Send is safe for I2c, but we're marked !Send because of the dummy pointer that's
// needed to force !Sync.
unsafe impl Send for I2c {}

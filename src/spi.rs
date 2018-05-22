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

//! Interface for the SPI peripherals.
//!
//! RPPAL provides access to the available SPI peripherals by using the `/dev/spidevB.C`
//! devices, where B points to an SPI bus (0, 1, 2), and C to a Chip Enable pin (0, 1, 2).
//!
//! The Raspberry Pi's GPIO header exposes several SPI buses. SPI0 is available
//! on all Raspberry Pi models. SPI1 is available on models with a 40-pin
//! header. SPI2 is only available on the Compute and Compute 3.
//!
//! SPI0 is disabled by default. You can enable it by running
//! `sudo raspi-config`, or by manually adding `dtparam=spi=on` to
//! `/boot/config.txt`. The associated pins are listed below.
//!
//! * MISO: BCM GPIO 9 (physical pin 21)
//! * MOSI: BCM GPIO 10 (physical pin 19)
//! * SCLK: BCM GPIO 11 (physical pin 23)
//! * SS: CE0: BCM GPIO 8 (physical pin 24), CE1: BCM GPIO 7 (physical pin 26)
//!
//! SPI1 is an auxiliary peripheral that's referred to as mini SPI. According
//! to the documentation, using higher clock speeds on SPI1 requires additional
//! CPU time compared to SPI0, caused by shallow FIFOs and no DMA support. SPI1
//! can be enabled by adding `dtoverlay=spi1-3cs` to `/boot/config.txt`. Replace
//! `3cs` with either `2cs` or `1cs` if you only require 2 or 1 Slave Select pins.
//! The associated pins are listed below.
//!
//! * MISO: BCM GPIO 19 (physical pin 35)
//! * MOSI: BCM GPIO 20 (physical pin 38)
//! * SCLK: BCM GPIO 21 (physical pin 40)
//! * SS: CE0: BCM GPIO 18 (physical pin 12), CE1: BCM GPIO 17 (physical pin 11), CE2: BCM GPIO 16 (physical pin 36)
//!
//! SPI2 shares the same characteristics as SPI1. It can be enabled by adding
//! `dtoverlay=spi2-3cs` to `/boot/config.txt`. Replace `3cs` with either `2cs` or
//! `1cs` if you only require 2 or 1 Slave Select pins. The associated pins are
//! listed below.
//!
//! * MISO: BCM GPIO 40
//! * MOSI: BCM GPIO 41
//! * SCLK: BCM GPIO 42
//! * SS: CE0: BCM GPIO 43, CE1: BCM GPIO 44, CE2: BCM GPIO 45
//!
//! The GPIO pin numbers mentioned above are part of the default configuration. Some of
//! their functionality can be moved to different pins. Read `/boot/overlays/README`
//! for more information.

use std::fs::{File, OpenOptions};
use std::io;
use std::io::{Read, Write};
use std::os::unix::io::AsRawFd;
use std::result;

use nix;

pub use nix::errno::Errno;

quick_error! {
    #[derive(Debug)]
/// Errors that can occur when accessing the SPI peripherals.
    pub enum Error {
/// IO error.
        Io(err: io::Error) { description(err.description()) from() }
/// System call error.
        Sys(errno: Errno) { display("System call error: {}", errno) }
/// Invalid path.
        InvalidPath {}
/// Invalid Utf8.
        InvalidUtf8 {}
/// Unsupported operation.
        UnsupportedOperation {}
    }
}

impl From<nix::Error> for Error {
    fn from(err: nix::Error) -> Error {
        match err {
            nix::Error::Sys(errno) => Error::Sys(errno),
            nix::Error::InvalidPath => Error::InvalidPath,
            nix::Error::InvalidUtf8 => Error::InvalidUtf8,
            nix::Error::UnsupportedOperation => Error::UnsupportedOperation,
        }
    }
}

/// Result type returned from methods that can have `spi::Error`s.
pub type Result<T> = result::Result<T, Error>;

mod ioctl {
    const SPI_IOC_MAGIC: u8 = b'k';

    const SPI_IOC_TYPE_MESSAGE: u8 = 0;
    const SPI_IOC_TYPE_LSB_FIRST: u8 = 2;
    const SPI_IOC_TYPE_BITS_PER_WORD: u8 = 3;
    const SPI_IOC_TYPE_MAX_SPEED_HZ: u8 = 4;
    const SPI_IOC_TYPE_MODE32: u8 = 5;

    #[derive(Debug, PartialEq, Copy, Clone)]
    #[repr(C)]
    pub struct TransferSegment {
        // Pointer to transmit buffer, or 0.
        tx_buf: u64,
        // Pointer to receive buffer, or 0.
        rx_buf: u64,
        // Number of bytes to transfer in this segment.
        len: u32,
        // Set a different clock speed for this segment. 0 = default.
        speed_hz: u32,
        // Add a delay before the (optional) SS change and the next segment.
        delay_usecs: u16,
        // Not used, since we only support 8 bits. 0 = default.
        bits_per_word: u8,
        // Set to 1 to briefly set SS inactive between this segment and the next, and keep SS active after the final segment.
        cs_change: u8,
        // Used for dual/quad SPI.
        tx_nbits: u8,
        // Used for dual/quad SPI.
        rx_nbits: u8,
        // Padding. Set to 0 for forward compatibility.
        pad: u16,
    }

    // TODO: Doublecheck cs_change behavior after the final transfer segment. Some SPI drivers may have
    // interpreted the documentation incorrectly.

    impl TransferSegment {
        pub fn new(read_buffer: Option<&mut [u8]>, write_buffer: Option<&[u8]>) -> TransferSegment {
            // Len will contain the length of the shortest of the supplied buffers
            let mut len: u32 = 0;

            let tx_buf = if let Some(buffer) = write_buffer {
                len = buffer.len() as u32;
                buffer.as_ptr() as u64
            } else {
                0
            };

            let rx_buf = if let Some(buffer) = read_buffer {
                if len > buffer.len() as u32 {
                    len = buffer.len() as u32;
                }
                buffer.as_ptr() as u64
            } else {
                0
            };

            TransferSegment {
                tx_buf,
                rx_buf,
                len,
                speed_hz: 0,
                delay_usecs: 0,
                bits_per_word: 0,
                cs_change: 0,
                tx_nbits: 0,
                rx_nbits: 0,
                pad: 0,
            }
        }

        pub fn len(&self) -> u32 {
            self.len
        }
    }

    ioctl!(write_buf spi_transfer with SPI_IOC_MAGIC, SPI_IOC_TYPE_MESSAGE; TransferSegment);
    ioctl!(write_int spi_write_mode with SPI_IOC_MAGIC, SPI_IOC_TYPE_MODE32);
    ioctl!(write_int spi_write_lsb_first with SPI_IOC_MAGIC, SPI_IOC_TYPE_LSB_FIRST);
    ioctl!(write_int spi_write_bits_per_word with SPI_IOC_MAGIC, SPI_IOC_TYPE_BITS_PER_WORD);
    ioctl!(write_int spi_write_max_speed_hz with SPI_IOC_MAGIC, SPI_IOC_TYPE_MAX_SPEED_HZ);
}

/// SPI buses.
///
/// The Raspberry Pi supports up to three SPI buses, depending on the model and
/// your `/boot/config.txt` configuration. More information can be found [here].
///
/// [here]: index.html
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Bus {
    Spi0 = 0,
    Spi1 = 1,
    Spi2 = 2,
}

/// Chip Enable (Slave Select) pins.
///
/// The Chip Enable pin is used to signal which device should
/// pay attention to the SPI bus. Chip Enable is more commonly
/// known as Slave Select or Chip Select.
///
/// The number of available Chip Enable pins for the selected SPI
/// bus depends on your `/boot/config.txt` configuration. More
/// information can be found [here].
///
/// [here]: index.html
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum ChipEnable {
    Ce0 = 0,
    Ce1 = 1,
    Ce2 = 2,
}

/// SPI modes.
///
/// Select the appropriate SPI mode for your device. Each mode
/// configures the clock polarity (CPOL) and clock phase (CPHA)
/// as shown below:
///
/// * Mode0: CPOL 0, CPHA 0
/// * Mode1: CPOL 0, CPHA 1
/// * Mode2: CPOL 1, CPHA 0
/// * Mode3: CPOL 1, CPHA 1
///
/// More information on clock polarity and phase can be found on [Wikipedia].
///
/// [Wikipedia]: https://en.wikipedia.org/wiki/Serial_Peripheral_Interface_Bus#Clock_polarity_and_phase
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Mode {
    Mode0 = 0,
    Mode1 = 1,
    Mode2 = 2,
    Mode3 = 3,
}

/// Bit order.
///
/// The bit order determines in what order data is shifted out and shifted in.
/// Select the bit order that's appropriate for the device you're communicating with.
///
/// `MsbFirst` will transfer the most-significant bit first. `LsbFirst` will transfer the
/// least-significant bit first.
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum BitOrder {
    MsbFirst = 0,
    LsbFirst = 1,
}

/// Provides access to the Raspberry Pi's SPI peripherals.
///
/// Before using `Spi`, make sure your Raspberry Pi has the necessary SPI buses and Chip Enable pins
/// enabled. More information can be found [here].
///
/// [here]: index.html
pub struct Spi {
    spidev: File,
}

impl Spi {
    /// Creates a new instance of `Spi`.
    ///
    /// `bus` and `chip_enable` specify the selected SPI bus and one of its associated Chip Enable pins.
    ///
    /// `clock_speed` defines the maximum clock speed in Hz. The SPI driver will automatically select
    /// the closest valid frequency.
    ///
    /// `mode` selects the clock polarity and phase.
    ///
    /// `bit_order` sets the order in which bits are shifted out and in to most-significant bit first or
    /// least-significant bit first.
    pub fn new(
        bus: Bus,
        chip_enable: ChipEnable,
        clock_speed: u32,
        mode: Mode,
        bit_order: BitOrder,
    ) -> Result<Spi> {
        // We don't ask for bits per word, because the driver only supports
        // 8 bits (or 9 bits in LoSSI mode). Changing the SS polarity from
        // active-low to active-high isn't supported either.
        // Based on https://www.raspberrypi.org/documentation/hardware/raspberrypi/spi/README.md
        // and https://www.kernel.org/doc/Documentation/spi/spidev.

        let spidev = OpenOptions::new()
            .read(true)
            .write(true)
            .open(format!("/dev/spidev{}.{}", bus as u8, chip_enable as u8))?;

        // Configure SPI bus through ioctl calls
        unsafe {
            ioctl::spi_write_mode(spidev.as_raw_fd(), mode as i32)?;
            ioctl::spi_write_max_speed_hz(spidev.as_raw_fd(), clock_speed as i32)?;
            ioctl::spi_write_bits_per_word(spidev.as_raw_fd(), 8)?;
            ioctl::spi_write_lsb_first(spidev.as_raw_fd(), bit_order as i32)?;
        }

        Ok(Spi { spidev })
    }

    /// Receives incoming data from the slave device and writes it to `buffer`.
    ///
    /// The SPI protocol doesn't indicate how much incoming data is waiting,
    /// so the maximum number of bytes read depends on the length of `buffer`.
    ///
    /// During the read, the MOSI line is kept in a state that results in a zero
    /// value byte shifted out for every byte `read` receives on the MISO line.
    ///
    /// Chip Enable is set to Low (active) at the start of the read, and
    /// High (inactive) when the read completes.
    ///
    /// Returns how many bytes were read.
    pub fn read(&mut self, buffer: &mut [u8]) -> Result<usize> {
        Ok(self.spidev.read(buffer)?)
    }

    /// Sends the outgoing data contained in `buffer` to the slave device.
    ///
    /// Any data received on the MISO line from the slave is ignored.
    ///
    /// Chip Enable is set to Low (active) at the start of the write, and
    /// High (inactive) when the write completes.
    ///
    /// Returns how many bytes were written.
    pub fn write(&mut self, buffer: &[u8]) -> Result<usize> {
        Ok(self.spidev.write(buffer)?)
    }

    /// Sends and receives data at the same time.
    ///
    /// SPI is a full-duplex protocol that shifts out bits to the slave device on the MOSI
    /// line while simultaneously shifting in bits it receives on the MISO line.
    /// `transfer` stores the incoming data in `read_buffer`, and sends the outgoing data
    /// contained in `write_buffer`.
    ///
    /// Because data is sent and received simultaneously, `transfer` only transfers
    /// as many bytes as the shortest of the two buffers contains.
    ///
    /// Chip Enable is set to Low (active) at the start of the transfer, and
    /// High (inactive) when the transfer completes.
    ///
    /// Returns how many bytes were transferred.
    pub fn transfer(&mut self, read_buffer: &mut [u8], write_buffer: &[u8]) -> Result<usize> {
        let segment = ioctl::TransferSegment::new(Some(read_buffer), Some(write_buffer));

        unsafe {
            ioctl::spi_transfer(self.spidev.as_raw_fd(), &[segment])?;
        }

        Ok(segment.len() as usize)
    }
}

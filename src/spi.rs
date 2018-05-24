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
//! Which of these buses and pins is available depends on your Raspberry Pi model and
//! configuration, as explained below.
//!
//! ## SPI buses
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
//! to the BCM2835 documentation, using higher clock speeds on SPI1 requires
//! additional CPU time compared to SPI0, caused by smaller FIFOs and no DMA
//! support. SPI1 can be enabled by adding `dtoverlay=spi1-3cs` to
//! `/boot/config.txt`. Replace `3cs` with either `2cs` or `1cs` if you only
//! require 2 or 1 Slave Select pins. The associated pins are listed below.
//!
//! * MISO: BCM GPIO 19 (physical pin 35)
//! * MOSI: BCM GPIO 20 (physical pin 38)
//! * SCLK: BCM GPIO 21 (physical pin 40)
//! * SS: CE0: BCM GPIO 18 (physical pin 12), CE1: BCM GPIO 17 (physical pin 11), CE2: BCM GPIO 16 (physical pin 36)
//!
//! SPI2 shares the same characteristics as SPI1. It can be enabled by adding
//! `dtoverlay=spi2-3cs` to `/boot/config.txt`. Replace `3cs` with either `2cs`
//! or `1cs` if you only require 2 or 1 Slave Select pins. The associated pins
//! are listed below.
//!
//! * MISO: BCM GPIO 40
//! * MOSI: BCM GPIO 41
//! * SCLK: BCM GPIO 42
//! * SS: CE0: BCM GPIO 43, CE1: BCM GPIO 44, CE2: BCM GPIO 45
//!
//! The GPIO pin numbers mentioned above are part of the default configuration.
//! Some of their functionality can be moved to different pins. Read
//! `/boot/overlays/README` for more information.
//!
//! ## Buffer size limits
//!
//! By default, spidev can handle up to 4096 bytes in a single
//! transfer. You can increase this limit to a maximum of 65536 bytes by adding
//! `spidev.bufsiz=65536` to the single line of parameters in `/boot/cmdline.txt`.
//! Remember to reboot the Raspberry Pi afterwards. The current value of bufsiz
//! can be checked with `cat /sys/module/spidev/parameters/bufsiz`.

use std::fs::{File, OpenOptions};
use std::io;
use std::io::{Read, Write};
use std::os::unix::io::AsRawFd;
use std::result;

use ioctl;

quick_error! {
    #[derive(Debug)]
/// Errors that can occur when accessing the SPI peripherals.
    pub enum Error {
/// IO error.
        Io(err: io::Error) { description(err.description()) from() }
/// The specified number of bits per word is not supported.
///
/// The Raspberry Pi currently only supports 8 bit words (or 9 bits in LoSSI
/// mode). Any other value will trigger this error.
        BitsPerWordNotSupported(bits_per_word: u8) { description("bits per word value not supported") }
/// The specified bit order is not supported.
///
/// The Raspberry Pi currently only supports the MsbFirst bit order. If you
/// need the LsbFirst bit order, you can use the [`reverse_bits`] function
/// instead to reverse the bit order in software by converting your write
/// buffer before sending it to the slave device, and your read buffer after
/// reading any incoming data.
///
/// [`reverse_bits`]: fn.reverse_bits.html
        BitOrderNotSupported(bit_order: BitOrder) { description("bit order value not supported") }
/// The specified clock speed is not supported.
        ClockSpeedNotSupported(clock_speed: u32) { description("clock speed value not supported") }
/// The specified mode is not supported.
        ModeNotSupported(mode: Mode) { description("mode value not supported") }
    }
}

/// Result type returned from methods that can have `spi::Error`s.
pub type Result<T> = result::Result<T, Error>;

const LOOKUP_REVERSE_BITS: [u8; 256] = [
    0x00, 0x80, 0x40, 0xC0, 0x20, 0xA0, 0x60, 0xE0, 0x10, 0x90, 0x50, 0xD0, 0x30, 0xB0, 0x70, 0xF0,
    0x08, 0x88, 0x48, 0xC8, 0x28, 0xA8, 0x68, 0xE8, 0x18, 0x98, 0x58, 0xD8, 0x38, 0xB8, 0x78, 0xF8,
    0x04, 0x84, 0x44, 0xC4, 0x24, 0xA4, 0x64, 0xE4, 0x14, 0x94, 0x54, 0xD4, 0x34, 0xB4, 0x74, 0xF4,
    0x0C, 0x8C, 0x4C, 0xCC, 0x2C, 0xAC, 0x6C, 0xEC, 0x1C, 0x9C, 0x5C, 0xDC, 0x3C, 0xBC, 0x7C, 0xFC,
    0x02, 0x82, 0x42, 0xC2, 0x22, 0xA2, 0x62, 0xE2, 0x12, 0x92, 0x52, 0xD2, 0x32, 0xB2, 0x72, 0xF2,
    0x0A, 0x8A, 0x4A, 0xCA, 0x2A, 0xAA, 0x6A, 0xEA, 0x1A, 0x9A, 0x5A, 0xDA, 0x3A, 0xBA, 0x7A, 0xFA,
    0x06, 0x86, 0x46, 0xC6, 0x26, 0xA6, 0x66, 0xE6, 0x16, 0x96, 0x56, 0xD6, 0x36, 0xB6, 0x76, 0xF6,
    0x0E, 0x8E, 0x4E, 0xCE, 0x2E, 0xAE, 0x6E, 0xEE, 0x1E, 0x9E, 0x5E, 0xDE, 0x3E, 0xBE, 0x7E, 0xFE,
    0x01, 0x81, 0x41, 0xC1, 0x21, 0xA1, 0x61, 0xE1, 0x11, 0x91, 0x51, 0xD1, 0x31, 0xB1, 0x71, 0xF1,
    0x09, 0x89, 0x49, 0xC9, 0x29, 0xA9, 0x69, 0xE9, 0x19, 0x99, 0x59, 0xD9, 0x39, 0xB9, 0x79, 0xF9,
    0x05, 0x85, 0x45, 0xC5, 0x25, 0xA5, 0x65, 0xE5, 0x15, 0x95, 0x55, 0xD5, 0x35, 0xB5, 0x75, 0xF5,
    0x0D, 0x8D, 0x4D, 0xCD, 0x2D, 0xAD, 0x6D, 0xED, 0x1D, 0x9D, 0x5D, 0xDD, 0x3D, 0xBD, 0x7D, 0xFD,
    0x03, 0x83, 0x43, 0xC3, 0x23, 0xA3, 0x63, 0xE3, 0x13, 0x93, 0x53, 0xD3, 0x33, 0xB3, 0x73, 0xF3,
    0x0B, 0x8B, 0x4B, 0xCB, 0x2B, 0xAB, 0x6B, 0xEB, 0x1B, 0x9B, 0x5B, 0xDB, 0x3B, 0xBB, 0x7B, 0xFB,
    0x07, 0x87, 0x47, 0xC7, 0x27, 0xA7, 0x67, 0xE7, 0x17, 0x97, 0x57, 0xD7, 0x37, 0xB7, 0x77, 0xF7,
    0x0F, 0x8F, 0x4F, 0xCF, 0x2F, 0xAF, 0x6F, 0xEF, 0x1F, 0x9F, 0x5F, 0xDF, 0x3F, 0xBF, 0x7F, 0xFF,
];

/// Reverses the bits of each byte in `buffer`.
///
/// Use this function to switch the bit order between most-significant bit first
/// and least-significant bit first.
#[inline(always)]
pub fn reverse_bits(buffer: &mut [u8]) {
    for byte in buffer {
        *byte = LOOKUP_REVERSE_BITS[*byte as usize];
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
#[repr(C)]
pub struct TransferSegment {
    // Pointer to transmit buffer, or 0.
    tx_buf: u64,
    // Pointer to receive buffer, or 0.
    rx_buf: u64,
    // Number of bytes to transfer in this segment.
    len: u32,
    // Set a different clock speed for this segment. Default = 0.
    speed_hz: u32,
    // Add a delay before the (optional) SS change and the next segment.
    delay_usecs: u16,
    // Not used, since we only support 8 bits (or 9 bits in LoSSI mode). Default = 0.
    bits_per_word: u8,
    // Set to 1 to briefly set SS High (inactive) between this segment and the next. If this is the last segment, keep SS Low (active).
    cs_change: u8,
    // Number of bits used for writing (dual/quad SPI). Default = 0.
    tx_nbits: u8,
    // Number of bits used for reading (dual/quad SPI). Default = 0.
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
/// The Chip Enable pin is used to signal which device should pay attention to
/// the SPI bus. Chip Enable is more commonly known as Slave Select or Chip
/// Select.
///
/// The number of available Chip Enable pins for the selected SPI bus depends
/// on your `/boot/config.txt` configuration. More information can be found
/// [here].
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
/// Select the appropriate SPI mode for your device. Each mode configures the
/// clock polarity (CPOL) and clock phase (CPHA) as shown below:
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
/// Select the bit order that's appropriate for the device you're
/// communicating with.
///
/// `MsbFirst` will transfer the most-significant bit first. `LsbFirst` will
/// transfer the least-significant bit first.
///
/// The Raspberry Pi currently only supports the MsbFirst bit order. If you
/// need the LsbFirst bit order, you can use the [`reverse_bits`] function
/// instead to reverse the bit order in software by converting your write
/// buffer before sending it to the slave device, and your read buffer after
/// reading any incoming data.
///
/// [`reverse_bits`]: fn.reverse_bits.html
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum BitOrder {
    MsbFirst = 0,
    LsbFirst = 1,
}

/// Provides access to the Raspberry Pi's SPI peripherals.
///
/// Before using `Spi`, make sure your Raspberry Pi has the necessary SPI buses
/// and Chip Enable pins enabled. More information can be found [here].
///
/// [here]: index.html
pub struct Spi {
    spidev: File,
}

impl Spi {
    /// Creates a new instance of `Spi`.
    ///
    /// `bus` and `chip_enable` specify the selected SPI bus and one of its
    /// associated Chip Enable pins.
    ///
    /// `clock_speed` defines the maximum clock speed in Hz. The SPI driver
    /// will automatically select the closest valid frequency.
    ///
    /// `mode` selects the clock polarity and phase.
    pub fn new(bus: Bus, chip_enable: ChipEnable, clock_speed: u32, mode: Mode) -> Result<Spi> {
        // We don't ask for bits per word here, because the driver only supports
        // 8 bits (or 9 bits in LoSSI mode). Changing the SS polarity from
        // active-low to active-high isn't supported. And the driver doesn't
        // support the LsbFirst bit order, so we don't explicitly ask for a bit
        // order either.
        // Based on https://www.raspberrypi.org/documentation/hardware/raspberrypi/spi/README.md
        // and https://www.kernel.org/doc/Documentation/spi/spidev.

        let spidev = OpenOptions::new()
            .read(true)
            .write(true)
            .open(format!("/dev/spidev{}.{}", bus as u8, chip_enable as u8))?;

        let spi = Spi { spidev };

        // Set defaults and user-specified settings
        spi.set_mode(mode)?;
        spi.set_clock_speed(clock_speed)?;
        spi.set_bits_per_word(8)?;
        spi.set_bit_order(BitOrder::MsbFirst)?;

        // TODO: (re)check support for CS_HIGH, NO_CS, 3WIRE, dual/quad SPI
        // From the rpi SPI doc page: Bidirectional or "3-wire" mode is supported
        // by the spi-bcm2835 kernel module. Please note that in this mode, either
        // the tx or rx field of the spi_transfer struct must be a NULL pointer,
        // since only half-duplex communication is possible. Otherwise, the transfer
        // will fail.

        Ok(spi)
    }

    /// Gets the bit order.
    pub fn bit_order(&self) -> Result<BitOrder> {
        let mut bit_order: u8 = 0;
        unsafe {
            ioctl::spidev::lsb_first(self.spidev.as_raw_fd(), &mut bit_order)?;
        }

        Ok(match bit_order {
            0 => BitOrder::MsbFirst,
            _ => BitOrder::LsbFirst,
        })
    }

    /// Sets the order in which bits are shifted out and in.
    ///
    /// By default, bit order is set to `MsbFirst`.
    ///
    /// The Raspberry Pi currently only supports the MsbFirst bit order. If you
    /// need the LsbFirst bit order, you can use the [`reverse_bits`] function
    /// instead to reverse the bit order in software by converting your write
    /// buffer before sending it to the slave device, and your read buffer after
    /// reading any incoming data.
    ///
    /// [`reverse_bits`]: fn.reverse_bits.html
    pub fn set_bit_order(&self, bit_order: BitOrder) -> Result<()> {
        match unsafe { ioctl::spidev::set_lsb_first(self.spidev.as_raw_fd(), bit_order as u8) } {
            Ok(_) => Ok(()),
            Err(ref e) if e.kind() == io::ErrorKind::InvalidInput => {
                Err(Error::BitOrderNotSupported(bit_order))
            }
            Err(e) => Err(Error::Io(e)),
        }
    }

    /// Gets the bits per word.
    pub fn bits_per_word(&self) -> Result<u8> {
        let mut bits_per_word: u8 = 0;
        unsafe {
            ioctl::spidev::bits_per_word(self.spidev.as_raw_fd(), &mut bits_per_word)?;
        }

        Ok(bits_per_word)
    }

    /// Sets the number of bits per word.
    ///
    /// By default, `bits_per_word` is set to 8.
    ///
    /// The Raspberry Pi currently only supports 8 bit words (or 9 bits in
    /// LoSSI mode).
    pub fn set_bits_per_word(&self, bits_per_word: u8) -> Result<()> {
        match unsafe { ioctl::spidev::set_bits_per_word(self.spidev.as_raw_fd(), bits_per_word) } {
            Ok(_) => Ok(()),
            Err(ref e) if e.kind() == io::ErrorKind::InvalidInput => {
                Err(Error::BitsPerWordNotSupported(bits_per_word))
            }
            Err(e) => Err(Error::Io(e)),
        }
    }

    /// Gets the clock speed.
    pub fn clock_speed(&self) -> Result<u32> {
        let mut clock_speed: u32 = 0;
        unsafe {
            ioctl::spidev::clock_speed(self.spidev.as_raw_fd(), &mut clock_speed)?;
        }

        Ok(clock_speed)
    }

    // Sets the clock speed frequency in Hz.
    pub fn set_clock_speed(&self, clock_speed: u32) -> Result<()> {
        match unsafe { ioctl::spidev::set_clock_speed(self.spidev.as_raw_fd(), clock_speed) } {
            Ok(_) => Ok(()),
            Err(ref e) if e.kind() == io::ErrorKind::InvalidInput => {
                Err(Error::ClockSpeedNotSupported(clock_speed))
            }
            Err(e) => Err(Error::Io(e)),
        }
    }

    // Gets the mode.
    pub fn mode(&self) -> Result<Mode> {
        let mut mode: u8 = 0;
        unsafe {
            ioctl::spidev::mode(self.spidev.as_raw_fd(), &mut mode)?;
        }

        Ok(match mode & 0x03 {
            0x01 => Mode::Mode1,
            0x02 => Mode::Mode2,
            0x03 => Mode::Mode3,
            _ => Mode::Mode0,
        })
    }

    // Sets the mode.
    pub fn set_mode(&self, mode: Mode) -> Result<()> {
        let mut new_mode: u8 = 0;
        unsafe {
            ioctl::spidev::mode(self.spidev.as_raw_fd(), &mut new_mode)?;
        }

        // Make sure we only replace the CPOL/CPHA bits
        new_mode = (new_mode & !0b11u8) | (mode as u8);

        match unsafe { ioctl::spidev::set_mode(self.spidev.as_raw_fd(), new_mode) } {
            Ok(_) => Ok(()),
            Err(ref e) if e.kind() == io::ErrorKind::InvalidInput => {
                Err(Error::ModeNotSupported(mode))
            }
            Err(e) => Err(Error::Io(e)),
        }
    }

    /// Receives incoming data from the slave device and writes it to `buffer`.
    ///
    /// The SPI protocol doesn't indicate how much incoming data is waiting,
    /// so the maximum number of bytes read depends on the length of `buffer`.
    ///
    /// During the read, the MOSI line is kept in a state that results in a
    /// zero value byte shifted out for every byte `read` receives on the MISO
    /// line.
    ///
    /// Chip Enable is set to Low (active) at the start of the read, and High
    /// (inactive) when the read completes.
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
    /// SPI is a full-duplex protocol that shifts out bits to the slave device
    /// on the MOSI line while simultaneously shifting in bits it receives on
    /// the MISO line. `transfer` stores the incoming data in `read_buffer`,
    /// and sends the outgoing data contained in `write_buffer`.
    ///
    /// Because data is sent and received simultaneously, `transfer` only
    /// transfers as many bytes as the shortest of the two buffers contains.
    ///
    /// Chip Enable is set to Low (active) at the start of the transfer, and
    /// High (inactive) when the transfer completes.
    ///
    /// Returns how many bytes were transferred.
    pub fn transfer(&mut self, read_buffer: &mut [u8], write_buffer: &[u8]) -> Result<usize> {
        let segment = TransferSegment::new(Some(read_buffer), Some(write_buffer));

        /*
        unsafe {
            ioctl::spidev::transfer(self.spidev.as_raw_fd(), &[segment])?;
        }
        */

        Ok(segment.len() as usize)
    }
}

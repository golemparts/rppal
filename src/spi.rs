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

//! Interface for the main and auxiliary SPI peripherals.
//!
//! RPPAL provides access to the available SPI buses by using the `spidev` device
//! interface through `/dev/spidevB.S`, where B points to an SPI bus (0, 1, 2), and S to
//! a Slave Select pin (0, 1, 2). Which of these buses and pins is available depends on
//! your Raspberry Pi model and configuration, as explained below.
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
//! * SS: [`Ss0`] BCM GPIO 8 (physical pin 24), [`Ss1`] BCM GPIO 7 (physical pin 26)
//!
//! SPI1 is an auxiliary peripheral that's referred to as mini SPI. According
//! to the BCM2835 documentation, using higher clock speeds on SPI1 requires
//! additional CPU time compared to SPI0, caused by smaller FIFOs and no DMA
//! support. It doesn't support [`Mode1`] or [`Mode3`]. SPI1 can be enabled by
//! adding `dtoverlay=spi1-3cs` to `/boot/config.txt`. Replace `3cs` with
//! either `2cs` or `1cs` if you only require 2 or 1 Slave Select pins.
//! The associated pins are listed below.
//!
//! * MISO: BCM GPIO 19 (physical pin 35)
//! * MOSI: BCM GPIO 20 (physical pin 38)
//! * SCLK: BCM GPIO 21 (physical pin 40)
//! * SS: [`Ss0`] BCM GPIO 18 (physical pin 12), [`Ss1`] BCM GPIO 17 (physical pin 11), [`Ss2`] BCM GPIO 16 (physical pin 36)
//!
//! SPI2 shares the same characteristics and limitations as SPI1. It can be
//! enabled by adding `dtoverlay=spi2-3cs` to `/boot/config.txt`. Replace
//! `3cs` with either `2cs` or `1cs` if you only require 2 or 1 Slave Select
//! pins. The associated pins are listed below.
//!
//! * MISO: BCM GPIO 40
//! * MOSI: BCM GPIO 41
//! * SCLK: BCM GPIO 42
//! * SS: [`Ss0`] BCM GPIO 43, [`Ss1`] BCM GPIO 44, [`Ss2`] BCM GPIO 45
//!
//! The GPIO pin numbers mentioned above are part of the default configuration.
//! Some of their functionality can be moved to different pins. Read
//! `/boot/overlays/README` for more information.
//!
//! ## Buffer size limits
//!
//! By default, `spidev` can handle up to 4096 bytes in a single transfer. You
//! can increase this limit to a maximum of 65536 bytes by appending
//! `spidev.bufsiz=65536` to the single line of parameters in `/boot/cmdline.txt`.
//! Remember to reboot the Raspberry Pi afterwards. The current value of bufsiz
//! can be checked with `cat /sys/module/spidev/parameters/bufsiz`.
//!
//! ## Not supported
//!
//! Some features exposed by the generic `spidev` interface aren't fully
//! supported by the underlying driver or the BCM283x SoC: `SPI_LSB_FIRST` (LSB
//! first bit order), `SPI_3WIRE` (bidirectional mode), `SPI_LOOP` (loopback mode),
//! `SPI_NO_CS` (no Slave Select), `SPI_READY` (slave ready signal),
//! `SPI_TX_DUAL`/`SPI_RX_DUAL` (dual SPI), `SPI_TX_QUAD`/`SPI_RX_QUAD` (quad SPI),
//! and any number of bits per word other than 8.
//!
//! If your slave device requires `SPI_LSB_FIRST`, you can use the
//! [`reverse_bits`] function instead to reverse the bit order in software.
//!
//! `SPI_LOOP` mode can be achieved by connecting the MOSI and MISO pins
//! together.
//!
//! `SPI_NO_CS` can be implemented by connecting the Slave Select pin on your
//! slave device to any other available GPIO pin on the Pi, and manually
//! changing it to high and low as needed.
//!
//! [`Ss0`]: enum.SlaveSelect.html
//! [`Ss1`]: enum.SlaveSelect.html
//! [`Ss2`]: enum.SlaveSelect.html
//! [`Mode1`]: enum.Mode.html
//! [`Mode3`]: enum.Mode.html
//! [`reverse_bits`]: fn.reverse_bits.html

use std::error;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io;
use std::io::{Read, Write};
use std::marker::PhantomData;
use std::os::unix::io::AsRawFd;
use std::result;

#[cfg(feature = "hal")]
mod hal;
mod ioctl;
mod segment;

pub use self::segment::Segment;

/// Errors that can occur when accessing the SPI peripheral.
#[derive(Debug)]
pub enum Error {
    /// I/O error.
    Io(io::Error),
    /// The specified number of bits per word is not supported.
    ///
    /// The Raspberry Pi currently only supports 8 bit words. Any other value
    /// will trigger this error.
    BitsPerWordNotSupported(u8),
    /// The specified bit order is not supported.
    ///
    /// The Raspberry Pi currently only supports the [`MsbFirst`] bit order. If you
    /// need the [`LsbFirst`] bit order, you can use the [`reverse_bits`] function
    /// instead to reverse the bit order in software by converting your write
    /// buffer before sending it to the slave device, and your read buffer after
    /// reading any incoming data.
    ///
    /// [`MsbFirst`]: enum.BitOrder.html
    /// [`LsbFirst`]: enum.BitOrder.html
    /// [`reverse_bits`]: fn.reverse_bits.html
    BitOrderNotSupported(BitOrder),
    /// The specified clock speed is not supported.
    ClockSpeedNotSupported(u32),
    /// The specified mode is not supported.
    ModeNotSupported(Mode),
    /// The specified Slave Select polarity is not supported.
    PolarityNotSupported(Polarity),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Error::Io(ref err) => write!(f, "I/O error: {}", err),
            Error::BitsPerWordNotSupported(bits_per_word) => {
                write!(f, "Bits per word value not supported: {}", bits_per_word)
            }
            Error::BitOrderNotSupported(bit_order) => {
                write!(f, "Bit order value not supported: {:?}", bit_order)
            }
            Error::ClockSpeedNotSupported(clock_speed) => {
                write!(f, "Clock speed value not supported: {}", clock_speed)
            }
            Error::ModeNotSupported(mode) => write!(f, "Mode value not supported: {:?}", mode),
            Error::PolarityNotSupported(polarity) => {
                write!(f, "Polarity value not supported: {:?}", polarity)
            }
        }
    }
}

impl error::Error for Error {}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
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

/// SPI buses.
///
/// The Raspberry Pi supports up to three SPI buses, depending on the model and
/// your `/boot/config.txt` configuration. More information can be found [here].
///
/// [here]: index.html
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Bus {
    Spi0 = 0,
    Spi1 = 1,
    Spi2 = 2,
}

impl fmt::Display for Bus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Bus::Spi0 => write!(f, "Spi0"),
            Bus::Spi1 => write!(f, "Spi1"),
            Bus::Spi2 => write!(f, "Spi2"),
        }
    }
}

/// Slave Select pins.
///
/// Slave Select is used to signal which slave device should pay attention to
/// the SPI bus. Slave Select (SS) is the more commonly used name, but
/// it's also known as Chip Select (CS) or Chip Enable (CE). Throughout the Raspberry
/// Pi's documentation, config files and BCM2835 datasheet, multiple different names
/// are used. Any pins referred to as CE0, CE1, and CE2 or CS0, CS1, and CS2 are equivalent
/// to `Ss0`, `Ss1`, and `Ss2`.
///
/// The number of available Slave Select pins for the selected SPI bus depends
/// on your `/boot/config.txt` configuration. More information can be found
/// [here].
///
/// [here]: index.html
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum SlaveSelect {
    Ss0 = 0,
    Ss1 = 1,
    Ss2 = 2,
}

impl fmt::Display for SlaveSelect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            SlaveSelect::Ss0 => write!(f, "Ss0"),
            SlaveSelect::Ss1 => write!(f, "Ss1"),
            SlaveSelect::Ss2 => write!(f, "Ss2"),
        }
    }
}

/// Slave Select polarities.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Polarity {
    ActiveLow = 0,
    ActiveHigh = 1,
}

impl fmt::Display for Polarity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Polarity::ActiveLow => write!(f, "ActiveLow"),
            Polarity::ActiveHigh => write!(f, "ActiveHigh"),
        }
    }
}

/// SPI modes indicating the clock polarity and phase.
///
/// Select the appropriate SPI mode for your device. Each mode configures the
/// clock polarity (CPOL) and clock phase (CPHA) as shown below:
///
/// * Mode0: CPOL 0, CPHA 0
/// * Mode1: CPOL 0, CPHA 1
/// * Mode2: CPOL 1, CPHA 0
/// * Mode3: CPOL 1, CPHA 1
///
/// The [`Spi0`] bus supports all 4 modes. [`Spi1`] and [`Spi2`] only support
/// `Mode0` and `Mode2`.
///
/// More information on clock polarity and phase can be found on [Wikipedia].
///
/// [`Spi0`]: enum.Bus.html
/// [`Spi1`]: enum.Bus.html
/// [`Spi2`]: enum.Bus.html
/// [Wikipedia]: https://en.wikipedia.org/wiki/Serial_Peripheral_Interface_Bus#Clock_polarity_and_phase
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Mode {
    Mode0 = 0,
    Mode1 = 1,
    Mode2 = 2,
    Mode3 = 3,
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Mode::Mode0 => write!(f, "Mode0"),
            Mode::Mode1 => write!(f, "Mode1"),
            Mode::Mode2 => write!(f, "Mode2"),
            Mode::Mode3 => write!(f, "Mode3"),
        }
    }
}

/// Bit orders.
///
/// The bit order determines in what order data is shifted out and shifted in.
/// Select the bit order that's appropriate for the device you're
/// communicating with.
///
/// `MsbFirst` will transfer the most-significant bit first. `LsbFirst` will
/// transfer the least-significant bit first.
///
/// The Raspberry Pi currently only supports the `MsbFirst` bit order. If you
/// need the `LsbFirst` bit order, you can use the [`reverse_bits`] function
/// instead to reverse the bit order in software by converting your write
/// buffer before sending it to the slave device, and your read buffer after
/// reading any incoming data.
///
/// [`reverse_bits`]: fn.reverse_bits.html
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum BitOrder {
    MsbFirst = 0,
    LsbFirst = 1,
}

impl fmt::Display for BitOrder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            BitOrder::MsbFirst => write!(f, "MsbFirst"),
            BitOrder::LsbFirst => write!(f, "LsbFirst"),
        }
    }
}

/// Provides access to the Raspberry Pi's SPI peripherals.
///
/// Before using `Spi`, make sure your Raspberry Pi has the necessary SPI buses
/// and Slave Select pins enabled. More information can be found [here].
///
/// The `embedded-hal` [`blocking::spi::Transfer<u8>`], [`blocking::spi::Write<u8>`]
/// and [`spi::FullDuplex<u8>`] trait
/// implementations for `Spi` can be enabled by specifying the optional `hal`
/// feature in the dependency declaration for the `rppal` crate.
///
/// [here]: index.html
/// [`blocking::spi::Transfer<u8>`]: ../../embedded_hal/blocking/spi/trait.Transfer.html
/// [`blocking::spi::Write<u8>`]: ../../embedded_hal/blocking/spi/trait.Write.html
/// [`spi::FullDuplex<u8>`]: ../../embedded_hal/spi/trait.FullDuplex.html
pub struct Spi {
    spidev: File,
    // Stores the last read value. Used for embedded_hal::spi::FullDuplex.
    #[cfg(feature = "hal")]
    last_read: u8,
    // The not_sync field is a workaround to force !Sync. Spi isn't safe for
    // Sync because of ioctl() and the underlying drivers. This avoids needing
    // #![feature(optin_builtin_traits)] to manually add impl !Sync for Spi.
    not_sync: PhantomData<*const ()>,
}

impl Spi {
    /// Constructs a new `Spi`.
    ///
    /// `bus` and `slave_select` specify the selected SPI bus and one of its
    /// associated Slave Select pins.
    ///
    /// `clock_speed` defines the maximum clock frequency in hertz (Hz). The SPI driver
    /// will automatically round down to the closest valid frequency.
    ///
    /// `mode` selects the clock polarity and phase.
    pub fn new(bus: Bus, slave_select: SlaveSelect, clock_speed: u32, mode: Mode) -> Result<Spi> {
        // The following options currently aren't supported by spidev in Raspbian Stretch on the Pi:
        //
        // LSB_FIRST - ioctl() returns EINVAL when set
        // 3WIRE - neither MOSI nor MISO show any outgoing data in half-duplex mode
        // LOOP - ioctl() returns EINVAL when set
        // NO_CS - SS is still set to active (tried both file write() and ioctl())
        // READY - ioctl() returns EINVAL when set
        // TX_DUAL/TX_QUAD/RX_DUAL/RX_QUAD - Not supported by BCM283x
        // bits per word - any value other than 0 or 8 returns EINVAL when set

        let spidev = OpenOptions::new()
            .read(true)
            .write(true)
            .open(format!("/dev/spidev{}.{}", bus as u8, slave_select as u8))?;

        // Reset all mode flags
        if let Err(e) = ioctl::set_mode32(spidev.as_raw_fd(), mode as u32) {
            if e.kind() == io::ErrorKind::InvalidInput {
                return Err(Error::ModeNotSupported(mode));
            } else {
                return Err(Error::Io(e));
            }
        }

        let spi = Spi {
            spidev,
            #[cfg(feature = "hal")]
            last_read: 0,
            not_sync: PhantomData,
        };

        // Set defaults and user-specified settings
        spi.set_bits_per_word(8)?;
        spi.set_clock_speed(clock_speed)?;

        Ok(spi)
    }

    /// Gets the bit order.
    pub fn bit_order(&self) -> Result<BitOrder> {
        let mut bit_order: u8 = 0;
        ioctl::lsb_first(self.spidev.as_raw_fd(), &mut bit_order)?;

        Ok(match bit_order {
            0 => BitOrder::MsbFirst,
            _ => BitOrder::LsbFirst,
        })
    }

    /// Sets the order in which bits are shifted out and in.
    ///
    /// The Raspberry Pi currently only supports the [`MsbFirst`] bit order. If you
    /// need the [`LsbFirst`] bit order, you can use the [`reverse_bits`] function
    /// instead to reverse the bit order in software by converting your write
    /// buffer before sending it to the slave device, and your read buffer after
    /// reading any incoming data.
    ///
    /// By default, `bit_order` is set to `MsbFirst`.
    ///
    /// [`MsbFirst`]: enum.BitOrder.html
    /// [`LsbFirst`]: enum.BitOrder.html
    /// [`reverse_bits`]: fn.reverse_bits.html
    pub fn set_bit_order(&self, bit_order: BitOrder) -> Result<()> {
        match ioctl::set_lsb_first(self.spidev.as_raw_fd(), bit_order as u8) {
            Ok(_) => Ok(()),
            Err(ref e) if e.kind() == io::ErrorKind::InvalidInput => {
                Err(Error::BitOrderNotSupported(bit_order))
            }
            Err(e) => Err(Error::Io(e)),
        }
    }

    /// Gets the number of bits per word.
    pub fn bits_per_word(&self) -> Result<u8> {
        let mut bits_per_word: u8 = 0;
        ioctl::bits_per_word(self.spidev.as_raw_fd(), &mut bits_per_word)?;

        Ok(bits_per_word)
    }

    /// Sets the number of bits per word.
    ///
    /// The Raspberry Pi currently only supports 8 bit words.
    ///
    /// By default, `bits_per_word` is set to 8.
    pub fn set_bits_per_word(&self, bits_per_word: u8) -> Result<()> {
        match ioctl::set_bits_per_word(self.spidev.as_raw_fd(), bits_per_word) {
            Ok(_) => Ok(()),
            Err(ref e) if e.kind() == io::ErrorKind::InvalidInput => {
                Err(Error::BitsPerWordNotSupported(bits_per_word))
            }
            Err(e) => Err(Error::Io(e)),
        }
    }

    /// Gets the clock frequency in hertz (Hz).
    pub fn clock_speed(&self) -> Result<u32> {
        let mut clock_speed: u32 = 0;
        ioctl::clock_speed(self.spidev.as_raw_fd(), &mut clock_speed)?;

        Ok(clock_speed)
    }

    /// Sets the clock frequency in hertz (Hz).
    ///
    /// The SPI driver will automatically round down to the closest valid frequency.
    pub fn set_clock_speed(&self, clock_speed: u32) -> Result<()> {
        match ioctl::set_clock_speed(self.spidev.as_raw_fd(), clock_speed) {
            Ok(_) => Ok(()),
            Err(ref e) if e.kind() == io::ErrorKind::InvalidInput => {
                Err(Error::ClockSpeedNotSupported(clock_speed))
            }
            Err(e) => Err(Error::Io(e)),
        }
    }

    /// Gets the SPI mode.
    pub fn mode(&self) -> Result<Mode> {
        let mut mode: u8 = 0;
        ioctl::mode(self.spidev.as_raw_fd(), &mut mode)?;

        Ok(match mode & 0x03 {
            0x01 => Mode::Mode1,
            0x02 => Mode::Mode2,
            0x03 => Mode::Mode3,
            _ => Mode::Mode0,
        })
    }

    /// Sets the SPI mode.
    ///
    /// The SPI mode indicates the serial clock polarity and phase. Some modes
    /// may not be available depending on the SPI bus that's used.
    pub fn set_mode(&self, mode: Mode) -> Result<()> {
        let mut new_mode: u8 = 0;
        ioctl::mode(self.spidev.as_raw_fd(), &mut new_mode)?;

        // Make sure we only replace the CPOL/CPHA bits
        new_mode = (new_mode & !0x03) | (mode as u8);

        match ioctl::set_mode(self.spidev.as_raw_fd(), new_mode) {
            Ok(_) => Ok(()),
            Err(ref e) if e.kind() == io::ErrorKind::InvalidInput => {
                Err(Error::ModeNotSupported(mode))
            }
            Err(e) => Err(Error::Io(e)),
        }
    }

    /// Gets the Slave Select polarity.
    pub fn ss_polarity(&self) -> Result<Polarity> {
        let mut mode: u8 = 0;
        ioctl::mode(self.spidev.as_raw_fd(), &mut mode)?;

        if (mode & ioctl::MODE_CS_HIGH) == 0 {
            Ok(Polarity::ActiveLow)
        } else {
            Ok(Polarity::ActiveHigh)
        }
    }

    /// Sets Slave Select polarity.
    ///
    /// By default, the Slave Select polarity is set to `ActiveLow`.
    pub fn set_ss_polarity(&self, polarity: Polarity) -> Result<()> {
        let mut new_mode: u8 = 0;
        ioctl::mode(self.spidev.as_raw_fd(), &mut new_mode)?;

        if polarity == Polarity::ActiveHigh {
            new_mode |= ioctl::MODE_CS_HIGH;
        } else {
            new_mode &= !ioctl::MODE_CS_HIGH;
        }

        match ioctl::set_mode(self.spidev.as_raw_fd(), new_mode) {
            Ok(_) => Ok(()),
            Err(ref e) if e.kind() == io::ErrorKind::InvalidInput => {
                Err(Error::PolarityNotSupported(polarity))
            }
            Err(e) => Err(Error::Io(e)),
        }
    }

    /// Receives incoming data from the slave device and writes it to `buffer`.
    ///
    /// The SPI protocol doesn't indicate how much incoming data is waiting,
    /// so the total number of bytes read depends on the length of `buffer`.
    ///
    /// During the read, the MOSI line is kept in a state that results in a
    /// zero value byte shifted out for every byte `read` receives on the MISO
    /// line.
    ///
    /// Slave Select is set to active at the start of the read, and inactive
    /// when the read completes.
    ///
    /// Returns how many bytes were read.
    pub fn read(&mut self, buffer: &mut [u8]) -> Result<usize> {
        Ok(self.spidev.read(buffer)?)
    }

    /// Sends the outgoing data contained in `buffer` to the slave device.
    ///
    /// Any data received on the MISO line from the slave is ignored.
    ///
    /// Slave Select is set to active at the start of the write, and inactive
    /// when the write completes.
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
    /// Because data is sent and received simultaneously, `transfer` will only
    /// transfer as many bytes as the shortest of the two buffers contains.
    ///
    /// Slave Select is set to active at the start of the transfer, and inactive
    /// when the transfer completes.
    ///
    /// Returns how many bytes were transferred.
    pub fn transfer(&self, read_buffer: &mut [u8], write_buffer: &[u8]) -> Result<usize> {
        let segment = Segment::new(read_buffer, write_buffer);

        ioctl::transfer(self.spidev.as_raw_fd(), &[segment])?;

        Ok(segment.len())
    }

    /// Transfers multiple half-duplex or full-duplex segments.
    ///
    /// `transfer_segments` transfers multiple segments in a single call. Each
    /// [`Segment`] contains a reference to either a read buffer or a write buffer,
    /// or both. Optional settings can be configured that override the SPI bus
    /// settings for that specific segment.
    ///
    /// By default, Slave Select stays active until all segments have been
    /// transferred. You can change this behavior using [`Segment::set_ss_change`].
    ///
    /// [`Segment`]: struct.Segment.html
    /// [`Segment::set_ss_change`]: struct.Segment.html#method.set_ss_change
    pub fn transfer_segments(&self, segments: &[Segment<'_, '_>]) -> Result<()> {
        ioctl::transfer(self.spidev.as_raw_fd(), segments)?;

        Ok(())
    }
}

// Send is safe for Spi, but we're marked !Send because of the dummy pointer that's
// needed to force !Sync.
unsafe impl Send for Spi {}

impl fmt::Debug for Spi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Spi").field("spidev", &self.spidev).finish()
    }
}

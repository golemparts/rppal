//! Interface for the main and auxiliary SPI peripherals.
//!
//! RPPAL provides access to the available SPI buses by using the `spidev` device
//! interface through `/dev/spidevB.S`, where B refers to an SPI bus, and S to
//! a Slave Select pin. Which buses and pins are available depends on your
//! Raspberry Pi model and configuration, as explained below.
//!
//! ## SPI buses
//!
//! The Raspberry Pi's GPIO header exposes several SPI buses. SPI0 is available
//! on all Raspberry Pi models. SPI1 is available on models with a 40-pin
//! header. SPI2 is only available on the Compute and Compute 3. SPI3 through SPI6
//! are only available on the Raspberry Pi 4 B and 400.
//!
//! ### SPI0
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
//! ### SPI1
//!
//! SPI1 is an auxiliary peripheral that's referred to as mini SPI. According
//! to the BCM2835 documentation, using higher clock speeds on SPI1 requires
//! additional CPU time compared to SPI0, caused by smaller FIFOs and no DMA
//! support. It doesn't support [`Mode1`] or [`Mode3`]. SPI1 can be enabled by
//! adding `dtoverlay=spi1-1cs` to `/boot/config.txt`. Replace `1cs` with
//! either `2cs` or `3cs` if you require 2 or 3 Slave Select pins.
//! The associated pins are listed below.
//!
//! * MISO: BCM GPIO 19 (physical pin 35)
//! * MOSI: BCM GPIO 20 (physical pin 38)
//! * SCLK: BCM GPIO 21 (physical pin 40)
//! * SS: [`Ss0`] BCM GPIO 18 (physical pin 12), [`Ss1`] BCM GPIO 17 (physical pin 11), [`Ss2`] BCM GPIO 16 (physical pin 36)
//!
//! ### SPI2
//!
//! SPI2 shares the same characteristics and limitations as SPI1. It can be
//! enabled by adding `dtoverlay=spi2-1cs` to `/boot/config.txt`. Replace
//! `1cs` with either `2cs` or `3cs` if you require 2 or 3 Slave Select
//! pins. The associated pins are listed below.
//!
//! * MISO: BCM GPIO 40
//! * MOSI: BCM GPIO 41
//! * SCLK: BCM GPIO 42
//! * SS: [`Ss0`] BCM GPIO 43, [`Ss1`] BCM GPIO 44, [`Ss2`] BCM GPIO 45
//!
//! ### SPI3
//!
//! SPI3 can be enabled by adding `dtoverlay=spi3-1cs` to `/boot/config.txt`. Replace
//! `1cs` with `2cs` if you require 2 Slave Select pins. The associated pins are listed below.
//!
//! * MISO: BCM GPIO 1 (physical pin 28)
//! * MOSI: BCM GPIO 2 (physical pin 3)
//! * SCLK: BCM GPIO 3 (physical pin 5)
//! * SS: [`Ss0`] BCM GPIO 0 (physical pin 27), [`Ss1`] BCM GPIO 24 (physical pin 18)
//!
//! ### SPI4
//!
//! SPI4 can be enabled by adding `dtoverlay=spi4-1cs` to `/boot/config.txt`. Replace
//! `1cs` with `2cs` if you require 2 Slave Select pins. The associated pins are listed below.
//!
//! * MISO: BCM GPIO 5 (physical pin 29)
//! * MOSI: BCM GPIO 6 (physical pin 31)
//! * SCLK: BCM GPIO 7 (physical pin 26)
//! * SS: [`Ss0`] BCM GPIO 4 (physical pin 7), [`Ss1`] BCM GPIO 25 (physical pin 22)
//!
//! ### SPI5
//!
//! SPI5 can be enabled by adding `dtoverlay=spi5-1cs` to `/boot/config.txt`. Replace
//! `1cs` with `2cs` if you require 2 Slave Select pins. The associated pins are listed below.
//!
//! * MISO: BCM GPIO 13 (physical pin 33)
//! * MOSI: BCM GPIO 14 (physical pin 8)
//! * SCLK: BCM GPIO 15 (physical pin 10)
//! * SS: [`Ss0`] BCM GPIO 12 (physical pin 32), [`Ss1`] BCM GPIO 26 (physical pin 37)
//!
//! ### SPI6
//!
//! SPI6 can be enabled by adding `dtoverlay=spi6-1cs` to `/boot/config.txt`. Replace
//! `1cs` with `2cs` if you require 2 Slave Select pins. The associated pins are listed below.
//!
//! * MISO: BCM GPIO 19 (physical pin 35)
//! * MOSI: BCM GPIO 20 (physical pin 38)
//! * SCLK: BCM GPIO 21 (physical pin 40)
//! * SS: [`Ss0`] BCM GPIO 18 (physical pin 12), [`Ss1`] BCM GPIO 27 (physical pin 13)
//!
//! SPI6 is tied to the same GPIO pins as SPI1. It's not possible to enable both
//! buses at the same time.
//!
//! ### Alternative pins
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
#[cfg(feature = "hal")]
pub use hal::SimpleHalSpiDevice;

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

/// Reverses the bits of each byte in `buffer`.
///
/// Use this function to switch the bit order between most-significant bit first
/// and least-significant bit first.
#[inline(always)]
pub fn reverse_bits(buffer: &mut [u8]) {
    for byte in buffer {
        *byte = byte.reverse_bits();
    }
}

/// SPI buses.
///
/// The Raspberry Pi exposes up to five SPI buses, depending on the model and
/// your `/boot/config.txt` configuration. More information can be found [here].
///
/// [here]: index.html
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Bus {
    Spi0 = 0,
    Spi1 = 1,
    Spi2 = 2,
    Spi3 = 3,
    Spi4 = 4,
    Spi5 = 5,
    Spi6 = 6,
}

impl fmt::Display for Bus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Bus::Spi0 => write!(f, "Spi0"),
            Bus::Spi1 => write!(f, "Spi1"),
            Bus::Spi2 => write!(f, "Spi2"),
            Bus::Spi3 => write!(f, "Spi3"),
            Bus::Spi4 => write!(f, "Spi4"),
            Bus::Spi5 => write!(f, "Spi5"),
            Bus::Spi6 => write!(f, "Spi6"),
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
    Ss3 = 3,
    Ss4 = 4,
    Ss5 = 5,
    Ss6 = 6,
    Ss7 = 7,
    Ss8 = 8,
    Ss9 = 9,
    Ss10 = 10,
    Ss11 = 11,
    Ss12 = 12,
    Ss13 = 13,
    Ss14 = 14,
    Ss15 = 15,
}

impl fmt::Display for SlaveSelect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            SlaveSelect::Ss0 => write!(f, "Ss0"),
            SlaveSelect::Ss1 => write!(f, "Ss1"),
            SlaveSelect::Ss2 => write!(f, "Ss2"),
            SlaveSelect::Ss3 => write!(f, "Ss3"),
            SlaveSelect::Ss4 => write!(f, "Ss4"),
            SlaveSelect::Ss5 => write!(f, "Ss5"),
            SlaveSelect::Ss6 => write!(f, "Ss6"),
            SlaveSelect::Ss7 => write!(f, "Ss7"),
            SlaveSelect::Ss8 => write!(f, "Ss8"),
            SlaveSelect::Ss9 => write!(f, "Ss9"),
            SlaveSelect::Ss10 => write!(f, "Ss10"),
            SlaveSelect::Ss11 => write!(f, "Ss11"),
            SlaveSelect::Ss12 => write!(f, "Ss12"),
            SlaveSelect::Ss13 => write!(f, "Ss13"),
            SlaveSelect::Ss14 => write!(f, "Ss14"),
            SlaveSelect::Ss15 => write!(f, "Ss15"),
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
    last_read: Option<u8>,
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
            last_read: None,
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

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

//! Interface for the SPI0 and SPI1 peripherals.

use std::fs::{File, OpenOptions};
use std::io;
use std::result;

quick_error! {
    #[derive(Debug)]
/// Errors that can occur when accessing the SPI peripherals.
    pub enum Error {
/// IO error.
        Io(err: io::Error) { description(err.description()) from() }
    }
}

/// Result type returned from methods that can have `spi::Error`s.
pub type Result<T> = result::Result<T, Error>;

mod ioctl {}

/// SPI buses.
///
/// The Raspberry Pi's GPIO header exposes several SPI buses. SPI0 is available
/// on all Raspberry Pi models. SPI1 is available on models with a 40-pin
/// header. SPI2 is only available on the Compute and Compute 3 module.
///
/// SPI0 is disabled by default. You can enable it by running
/// `sudo raspi-config`, or by manually adding `dtparam=spi=on` to
/// `/boot/config.txt`. The associated pins are listed below.
///
/// * MISO: BCM GPIO 9 (physical pin 21)
/// * MOSI: BCM GPIO 10 (physical pin 19)
/// * SCLK: BCM GPIO 11 (physical pin 23)
/// * SS: CE0: BCM GPIO 8 (physical pin 24), CE1: BCM GPIO 7 (physical pin 26)
///
/// SPI1 is an auxiliary peripheral that's referred to as mini SPI. According
/// to the documentation, using higher clock speeds on SPI1 requires additional
/// CPU time compared to SPI0, caused by shallow FIFOs and no DMA support. SPI1
/// can be enabled by adding `dtoverlay=spi1-3cs` to `/boot/config.txt`. Replace
/// `3cs` with either `2cs` or `1cs` if you only require 2 or 1 Slave Select pins.
/// The associated pins are listed below.
///
/// * MISO: BCM GPIO 19 (physical pin 35)
/// * MOSI: BCM GPIO 20 (physical pin 38)
/// * SCLK: BCM GPIO 21 (physical pin 40)
/// * SS: CE0: BCM GPIO 18 (physical pin 12), CE1: BCM GPIO 17 (physical pin 11), CE2: BCM GPIO 16 (physical pin 36)
///
/// SPI2 shares the same characteristics as SPI1. It can be enabled by adding
/// `dtoverlay=spi2-3cs` to `/boot/config.txt`. Replace `3cs` with either `2cs` or
/// `1cs` if you only require 2 or 1 Slave Select pins. The associated pins are
/// listed below.
///
/// * MISO: BCM GPIO 40
/// * MOSI: BCM GPIO 41
/// * SCLK: BCM GPIO 42
/// * SS: CE0: BCM GPIO 43, CE1: BCM GPIO 44, CE2: BCM GPIO 45
///
/// The GPIO pin numbers mentioned above are part of the default configuration. Some of
/// them can be moved to different pins. Read `/boot/overlays/README` for more information.
pub enum Bus {
    Spi0 = 0,
    Spi1 = 1,
    Spi2 = 2,
}

/// Chip Enable (Slave Select) pins.
///
/// Select a Chip Enable pin to signal which device should
/// interact with the SPI master. Chip Enable is more commonly
/// known as Slave Select or Chip Select.
///
/// Each of the available SPI buses has access to either two
/// or three Chip Enable pins, as shown below:
///
/// * Spi0: Ce0 (BCM GPIO 8), Ce1 (BCM GPIO 7)
/// * Spi1: Ce0 (BCM GPIO 18, Ce1 (BCM GPIO 17), Ce2 (BCM GPIO 16)
/// * Spi2: Ce0 (BCM GPIO 43, Ce1 (BCM GPIO 44), Ce2 (BCM GPIO 45)
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
pub enum Mode {
    Mode0 = 0, // CPOL 0, CPHA 0
    Mode1 = 1, // CPOL 0, CPHA 1
    Mode2 = 2, // CPOL 1, CPHA 0
    Mode3 = 3, // CPOL 1, CPHA 1
}

pub struct Spi {
    spidev: File,
}

impl Spi {
    pub fn new(bus: Bus, chip_enable: ChipEnable, mode: Mode, speed: u32) -> Result<Spi> {
        let spidev = OpenOptions::new()
            .read(true)
            .write(true)
            .open(format!("/dev/spidev{}.{}", bus as u8, chip_enable as u8))?;

        Ok(Spi { spidev })
    }

    pub fn read(&self, mut buffer: &[u8]) -> Result<()> {
        Ok(())
    }

    pub fn write(&self, buffer: &[u8]) -> Result<()> {
        Ok(())
    }

    pub fn transfer(&self, mut read_buffer: &[u8], write_buffer: &[u8]) -> Result<()> {
        Ok(())
    }
}

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

use std::fs::{File, OpenOptions};
use std::io;
use std::result;

quick_error! {
    #[derive(Debug)]
    pub enum Error {
/// IO error.
        Io(err: io::Error) { description(err.description()) from() }
    }
}

/// Result type returned from methods that can have `spi::Error`s.
pub type Result<T> = result::Result<T, Error>;

mod ioctl {}

pub enum Device {
    Spi0 = 0, // Enabled by default
    Spi1 = 1, // Requires additional configuration
}

pub enum ChipEnable {
    Ce0 = 0, // SPI0: BCM GPIO 8    SPI1: BCM GPIO 18
    Ce1 = 1, // SPI0: BCM GPIO 7    SPI1: BCM GPIO 17
    Ce2 = 2, // SPI0: N/A,          SPI1: BCM GPIO 16
}

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
    pub fn new(device: Device, chip_enable: ChipEnable, mode: Mode, speed: u32) -> Result<Spi> {
        let spidev = OpenOptions::new().read(true).write(true).open(format!(
            "/dev/spidev{}.{}",
            device as u8, chip_enable as u8
        ))?;

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

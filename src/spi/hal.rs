#![allow(clippy::needless_lifetimes)]

use embedded_hal::spi::nb::FullDuplex;
use embedded_hal::spi::{
    self,
    blocking::{Transfer, Write},
    ErrorType,
};

use super::{Error, Spi};

impl ErrorType for Spi {
    type Error = Error;
}

impl spi::Error for Error {
    fn kind(&self) -> spi::ErrorKind {
        spi::ErrorKind::Other
    }
}

/// `Transfer<u8>` trait implementation for `embedded-hal` v1.0.0-alpha.7.
impl Transfer<u8> for Spi {
    fn transfer<'a>(&mut self, read: &'a mut [u8], write: &[u8]) -> Result<(), Self::Error> {
        Spi::transfer(self, read, write)?;

        Ok(())
    }
}

/// `Transfer<u8>` trait implementation for `embedded-hal` v0.2.6.
impl embedded_hal_0::blocking::spi::Transfer<u8> for Spi {
    type Error = Error;

    fn transfer<'a>(&mut self, buffer: &'a mut [u8]) -> Result<&'a [u8], Self::Error> {
        let write_buffer = buffer.to_vec();
        Transfer::transfer(self, buffer, &write_buffer)?;
        Ok(buffer)
    }
}

/// `Write<u8>` trait implementation for `embedded-hal` v1.0.0-alpha.7.
impl Write<u8> for Spi {
    fn write(&mut self, buffer: &[u8]) -> Result<(), Self::Error> {
        Spi::write(self, buffer)?;

        Ok(())
    }
}

/// `Write<u8>` trait implementation for `embedded-hal` v0.2.6.
impl embedded_hal_0::blocking::spi::Write<u8> for Spi {
    type Error = Error;

    fn write(&mut self, buffer: &[u8]) -> Result<(), Self::Error> {
        Write::write(self, buffer)
    }
}

/// `FullDuplex<u8>` trait implementation for `embedded-hal` v1.0.0-alpha.7.
impl FullDuplex<u8> for Spi {
    fn read(&mut self) -> nb::Result<u8, Self::Error> {
        if let Some(last_read) = self.last_read.take() {
            Ok(last_read)
        } else {
            Err(nb::Error::WouldBlock)
        }
    }

    fn write(&mut self, byte: u8) -> nb::Result<(), Self::Error> {
        let mut read_buffer: [u8; 1] = [0];

        Spi::transfer(self, &mut read_buffer, &[byte])?;
        self.last_read = Some(read_buffer[0]);

        Ok(())
    }
}

/// `FullDuplex<u8>` trait implementation for `embedded-hal` v0.2.6.
impl embedded_hal_0::spi::FullDuplex<u8> for Spi {
    type Error = Error;

    fn read(&mut self) -> nb::Result<u8, Self::Error> {
        FullDuplex::read(self)
    }

    fn send(&mut self, byte: u8) -> nb::Result<(), Self::Error> {
        FullDuplex::write(self, byte)
    }
}

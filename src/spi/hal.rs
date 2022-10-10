use embedded_hal::spi::nb::FullDuplex;
use embedded_hal::spi::{
    self,
    blocking::{SpiDevice, SpiBus, SpiBusRead, SpiBusWrite, SpiBusFlush},
    ErrorType,
};
use std::io;

use super::{Error, Spi};

impl ErrorType for Spi {
    type Error = Error;
}

impl spi::Error for Error {
    fn kind(&self) -> spi::ErrorKind {
        spi::ErrorKind::Other
    }
}

/// `Transfer<u8>` trait implementation for `embedded-hal` v1.0.0-alpha.8.
impl SpiBus<u8> for Spi {
    fn transfer(&mut self, read: &mut [u8], write: &[u8]) -> Result<(), Self::Error> {
        Spi::transfer(self, read, write)?;

        Ok(())
    }

    fn transfer_in_place(&mut self, buffer: &mut [u8]) -> Result<(), Self::Error> {
        let write_buffer = buffer.to_vec();
        self.transfer(buffer, &write_buffer)
    }
}

/// `Transfer<u8>` trait implementation for `embedded-hal` v0.2.6.
impl embedded_hal_0::blocking::spi::Transfer<u8> for Spi {
    type Error = Error;

    fn transfer<'a>(&mut self, buffer: &'a mut [u8]) -> Result<&'a [u8], Self::Error> {
        let write_buffer = buffer.to_vec();
        SpiBus::transfer(self, buffer, &write_buffer)?;
        Ok(buffer)
    }
}

/// `SpiBusWrite<u8>` trait implementation for `embedded-hal` v1.0.0-alpha.8.
impl SpiBusWrite<u8> for Spi {
    fn write(&mut self, buffer: &[u8]) -> Result<(), Self::Error> {
        Spi::write(self, buffer)?;

        Ok(())
    }
}

/// `Write<u8>` trait implementation for `embedded-hal` v0.2.6.
impl embedded_hal_0::blocking::spi::Write<u8> for Spi {
    type Error = Error;

    fn write(&mut self, buffer: &[u8]) -> Result<(), Self::Error> {
        SpiBusWrite::write(self, buffer)
    }
}

/// `SpiBusRead<u8>` trait implementation for `embedded-hal` v1.0.0-alpha.8.
impl SpiBusRead<u8> for Spi {
    fn read(&mut self, buffer: &mut [u8]) -> Result<(), Self::Error> {
        Spi::read(self, buffer)?;

        Ok(())
    }
}

/// `SpiBusFlush` trait implementation for `embedded-hal` v1.0.0-alpha.8.
impl SpiBusFlush for Spi {
    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

/// `FullDuplex<u8>` trait implementation for `embedded-hal` v1.0.0-alpha.8.
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

/// Simple implementation of [embedded_hal::spi::blocking::SpiDevice]
///
/// You only need this when using the `embedded_hal` Spi trait interface.
///
/// Slave-select is currently handled at the bus level.
/// This no-op device implementation can be used to satisfy the trait.
// TODO: The underlying crate::spi::Spi shall be split up to support proper slave-select handling here.
pub struct SimpleHalSpiDevice<B> {
    bus: B,
}

impl<B> SimpleHalSpiDevice<B> {
    pub fn new(bus: B) -> SimpleHalSpiDevice<B> {
        SimpleHalSpiDevice { bus }
    }
}

impl<B: ErrorType> SpiDevice for SimpleHalSpiDevice<B> {
    type Bus = B;

    fn transaction<R>(
        &mut self,
        f: impl FnOnce(&mut Self::Bus) -> Result<R, <Self::Bus as ErrorType>::Error>
    ) -> Result<R, Self::Error> {
        f(&mut self.bus)
            .map_err(|_| Error::Io(io::Error::new(io::ErrorKind::Other, "SimpleHalSpiDevice transaction error")))
    }
}

impl<B: ErrorType> ErrorType for SimpleHalSpiDevice<B> {
    type Error = Error;
}

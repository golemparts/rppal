use embedded_hal::i2c::{self, ErrorType, I2c as I2cHal, Operation as I2cOperation};

use super::{Error, I2c};

/// `Write` trait implementation for `embedded-hal` v0.2.7.
impl embedded_hal_0::blocking::i2c::Write for I2c {
    type Error = Error;

    fn write(&mut self, address: u8, bytes: &[u8]) -> Result<(), Self::Error> {
        I2cHal::write(self, address, bytes)
    }
}

/// `Read` trait implementation for `embedded-hal` v0.2.7.
impl embedded_hal_0::blocking::i2c::Read for I2c {
    type Error = Error;

    fn read(&mut self, address: u8, buffer: &mut [u8]) -> Result<(), Self::Error> {
        I2cHal::read(self, address, buffer)
    }
}

/// `WriteRead` trait implementation for `embedded-hal` v0.2.7.
impl embedded_hal_0::blocking::i2c::WriteRead for I2c {
    type Error = Error;

    fn write_read(
        &mut self,
        address: u8,
        bytes: &[u8],
        buffer: &mut [u8],
    ) -> Result<(), Self::Error> {
        I2cHal::write_read(self, address, bytes, buffer)
    }
}

impl ErrorType for I2c {
    type Error = Error;
}

impl i2c::Error for Error {
    fn kind(&self) -> i2c::ErrorKind {
        if let Error::Io(e) = self {
            use std::io::ErrorKind::*;

            match e.kind() {
                /* ResourceBusy | */ InvalidData => i2c::ErrorKind::Bus,
                WouldBlock => i2c::ErrorKind::ArbitrationLoss,
                _ => i2c::ErrorKind::Other,
            }
        } else {
            i2c::ErrorKind::Other
        }
    }
}

/// `I2c` trait implementation for `embedded-hal` v1.0.0.
impl I2cHal for I2c {
    fn transaction(
        &mut self,
        address: u8,
        operations: &mut [I2cOperation],
    ) -> Result<(), Self::Error> {
        self.set_slave_address(u16::from(address))?;
        for op in operations {
            match op {
                I2cOperation::Read(buff) => {
                    I2c::read(self, buff)?;
                }
                I2cOperation::Write(buff) => {
                    I2c::write(self, buff)?;
                }
            }
        }

        Ok(())
    }
}

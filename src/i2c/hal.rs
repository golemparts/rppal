use super::{Error, I2c};

#[cfg(feature = "embedded-hal-0")]
impl embedded_hal_0::blocking::i2c::Write for I2c {
    type Error = Error;

    fn write(&mut self, address: u8, bytes: &[u8]) -> Result<(), Self::Error> {
        embedded_hal::i2c::I2c::write(self, address, bytes)
    }
}

#[cfg(feature = "embedded-hal-0")]
impl embedded_hal_0::blocking::i2c::Read for I2c {
    type Error = Error;

    fn read(&mut self, address: u8, buffer: &mut [u8]) -> Result<(), Self::Error> {
        embedded_hal::i2c::I2c::read(self, address, buffer)
    }
}

#[cfg(feature = "embedded-hal-0")]
impl embedded_hal_0::blocking::i2c::WriteRead for I2c {
    type Error = Error;

    fn write_read(
        &mut self,
        address: u8,
        bytes: &[u8],
        buffer: &mut [u8],
    ) -> Result<(), Self::Error> {
        embedded_hal::i2c::I2c::write_read(self, address, bytes, buffer)
    }
}

#[cfg(feature = "embedded-hal")]
impl embedded_hal::i2c::ErrorType for I2c {
    type Error = Error;
}

#[cfg(feature = "embedded-hal")]
impl embedded_hal::i2c::Error for Error {
    fn kind(&self) -> embedded_hal::i2c::ErrorKind {
        if let Error::Io(e) = self {
            use std::io::ErrorKind::*;

            match e.kind() {
                /* ResourceBusy | */ InvalidData => embedded_hal::i2c::ErrorKind::Bus,
                WouldBlock => embedded_hal::i2c::ErrorKind::ArbitrationLoss,
                _ => embedded_hal::i2c::ErrorKind::Other,
            }
        } else {
            embedded_hal::i2c::ErrorKind::Other
        }
    }
}

#[cfg(feature = "embedded-hal")]
impl embedded_hal::i2c::I2c for I2c {
    fn transaction(
        &mut self,
        address: u8,
        operations: &mut [embedded_hal::i2c::Operation],
    ) -> Result<(), Self::Error> {
        self.set_slave_address(u16::from(address))?;
        for op in operations {
            match op {
                embedded_hal::i2c::Operation::Read(buff) => {
                    self.read(buff)?;
                }
                embedded_hal::i2c::Operation::Write(buff) => {
                    self.write(buff)?;
                }
            }
        }

        Ok(())
    }
}

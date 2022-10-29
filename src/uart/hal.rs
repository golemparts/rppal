use embedded_hal::serial::{
    self,
    ErrorType,
};
use embedded_hal_nb::serial::{
    Read,
    Write,
};

use super::{Error, Queue, Uart};

impl ErrorType for Uart {
    type Error = Error;
}

impl serial::Error for Error {
    fn kind(&self) -> serial::ErrorKind {
        serial::ErrorKind::Other
    }
}

/// `Read<u8>` trait implementation for `embedded-hal` v1.0.0-alpha.7.
impl Read<u8> for Uart {
    fn read(&mut self) -> nb::Result<u8, Self::Error> {
        let mut buffer = [0u8; 1];
        if Uart::read(self, &mut buffer)? == 0 {
            Err(nb::Error::WouldBlock)
        } else {
            Ok(buffer[0])
        }
    }
}

/// `Read<u8>` trait implementation for `embedded-hal` v0.2.6.
impl embedded_hal_0::serial::Read<u8> for Uart {
    type Error = Error;

    fn read(&mut self) -> nb::Result<u8, Self::Error> {
        Read::read(self)
    }
}

/// `Write<u8>` trait implementation for `embedded-hal` v1.0.0-alpha.7.
impl Write<u8> for Uart {
    fn write(&mut self, word: u8) -> nb::Result<(), Self::Error> {
        if Uart::write(self, &[word])? == 0 {
            Err(nb::Error::WouldBlock)
        } else {
            Ok(())
        }
    }

    fn flush(&mut self) -> nb::Result<(), Self::Error> {
        Uart::flush(self, Queue::Output)?;

        Ok(())
    }
}

/// `Write<u8>` trait implementation for `embedded-hal` v0.2.6.
impl embedded_hal_0::serial::Write<u8> for Uart {
    type Error = Error;

    fn write(&mut self, word: u8) -> nb::Result<(), Self::Error> {
        Write::write(self, word)
    }

    fn flush(&mut self) -> nb::Result<(), Self::Error> {
        Write::flush(self)
    }
}

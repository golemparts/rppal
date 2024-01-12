#[cfg(any(feature = "embedded-hal-0", feature = "embedded-hal-nb"))]
use super::{Error, Queue, Uart};

#[cfg(feature = "embedded-hal-nb")]
impl embedded_hal_nb::serial::ErrorType for Uart {
    type Error = Error;
}

#[cfg(feature = "embedded-hal-nb")]
impl embedded_hal_nb::serial::Error for Error {
    fn kind(&self) -> embedded_hal_nb::serial::ErrorKind {
        embedded_hal_nb::serial::ErrorKind::Other
    }
}

#[cfg(feature = "embedded-hal-nb")]
impl embedded_hal_nb::serial::Read<u8> for Uart {
    fn read(&mut self) -> embedded_hal_nb::nb::Result<u8, Self::Error> {
        let mut buffer = [0u8; 1];
        if Uart::read(self, &mut buffer)? == 0 {
            Err(embedded_hal_nb::nb::Error::WouldBlock)
        } else {
            Ok(buffer[0])
        }
    }
}

#[cfg(feature = "embedded-hal-0")]
impl embedded_hal_0::serial::Read<u8> for Uart {
    type Error = Error;

    fn read(&mut self) -> nb::Result<u8, Self::Error> {
        embedded_hal_nb::serial::Read::read(self)
    }
}

#[cfg(feature = "embedded-hal-nb")]
impl embedded_hal_nb::serial::Write<u8> for Uart {
    fn write(&mut self, word: u8) -> embedded_hal_nb::nb::Result<(), Self::Error> {
        if Uart::write(self, &[word])? == 0 {
            Err(embedded_hal_nb::nb::Error::WouldBlock)
        } else {
            Ok(())
        }
    }

    fn flush(&mut self) -> embedded_hal_nb::nb::Result<(), Self::Error> {
        Uart::flush(self, Queue::Output)?;

        Ok(())
    }
}

#[cfg(feature = "embedded-hal-0")]
impl embedded_hal_0::serial::Write<u8> for Uart {
    type Error = Error;

    fn write(&mut self, word: u8) -> nb::Result<(), Self::Error> {
        embedded_hal_nb::serial::Write::write(self, word)
    }

    fn flush(&mut self) -> nb::Result<(), Self::Error> {
        embedded_hal_nb::serial::Write::flush(self)
    }
}

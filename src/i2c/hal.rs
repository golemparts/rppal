// Copyright (c) 2017-2021 Rene van der Meer
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

use embedded_hal::i2c::{self, ErrorType, blocking::{I2c as I2cHal, Operation as I2cOperation}};

use super::{Error, I2c};

/// `Write` trait implementation for `embedded-hal` v0.2.6.
impl embedded_hal_0::blocking::i2c::Write for I2c {
    type Error = Error;

    fn write(&mut self, address: u8, bytes: &[u8]) -> Result<(), Self::Error> {
        I2cHal::write(self, address, bytes)
    }
}

/// `Read` trait implementation for `embedded-hal` v0.2.6.
impl embedded_hal_0::blocking::i2c::Read for I2c {
    type Error = Error;

    fn read(&mut self, address: u8, buffer: &mut [u8]) -> Result<(), Self::Error> {
        I2cHal::read(self, address, buffer)
    }
}

/// `WriteRead` trait implementation for `embedded-hal` v0.2.6.
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

/// `I2c` trait implementation for `embedded-hal` v1.0.0-alpha.7.
impl I2cHal for I2c {
    fn write(&mut self, address: u8, bytes: &[u8]) -> Result<(), Self::Error> {
        self.set_slave_address(u16::from(address))?;
        I2c::write(self, bytes)?;

        Ok(())
    }

    fn read(&mut self, address: u8, buffer: &mut [u8]) -> Result<(), Self::Error> {
        self.set_slave_address(u16::from(address))?;
        I2c::read(self, buffer)?;

        Ok(())
    }

    fn write_iter<B>(&mut self, address: u8, bytes: B) -> Result<(), Self::Error>
    where
        B: IntoIterator<Item = u8> {
        let bytes: Vec<_> = bytes.into_iter().collect();
        I2cHal::write(self, address, &bytes)
    }

    fn write_read(
        &mut self,
        address: u8,
        bytes: &[u8],
        buffer: &mut [u8],
    ) -> Result<(), Self::Error> {
        self.set_slave_address(u16::from(address))?;
        I2c::write_read(self, bytes, buffer)?;

        Ok(())
    }

    fn write_iter_read<B>(
        &mut self,
        address: u8,
        bytes: B,
        buffer: &mut [u8],
    ) -> Result<(), Self::Error>
    where
        B: IntoIterator<Item = u8>,
    {
        let bytes: Vec<_> = bytes.into_iter().collect();
        self.transaction(
            address,
            &mut [I2cOperation::Write(&bytes), I2cOperation::Read(buffer)],
        )
    }

    fn transaction(
        &mut self,
        _address: u8,
        _operations: &mut [I2cOperation],
    ) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn transaction_iter<'a, O>(&mut self, address: u8, operations: O) -> Result<(), Self::Error>
    where
        O: IntoIterator<Item = I2cOperation<'a>>,
    {
        let mut ops: Vec<_> = operations.into_iter().collect();
        self.transaction(address, &mut ops)
    }
}

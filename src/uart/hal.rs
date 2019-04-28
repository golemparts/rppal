// Copyright (c) 2017-2019 Rene van der Meer
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

use embedded_hal::blocking::serial::write::Default;
use embedded_hal::serial::{Read, Write};
use nb;

use super::{Error, Queue, Uart};

impl Read<u8> for Uart {
    type Error = Error;

    fn read(&mut self) -> nb::Result<u8, Self::Error> {
        let mut buffer = [0u8; 1];
        if Uart::read(self, &mut buffer)? == 0 {
            Err(nb::Error::WouldBlock)
        } else {
            Ok(buffer[0])
        }
    }
}

impl Write<u8> for Uart {
    type Error = Error;

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

impl Default<u8> for Uart {}

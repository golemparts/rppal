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

#![allow(dead_code)]

use libc::{c_int, c_ulong, ioctl};
use std::io;
use std::result;

pub type Result<T> = result::Result<T, io::Error>;

fn parse_retval(retval: c_int) -> Result<i32> {
    if retval == -1 {
        Err(io::Error::last_os_error())
    } else {
        Ok(retval)
    }
}

const REQ_RETRIES: c_ulong = 0x0701; // How many retries when waiting for an ACK
const REQ_TIMEOUT: c_ulong = 0x0702; // Timeout in 10ms units
const REQ_SLAVE: c_ulong = 0x0706; // Set slave address
const REQ_SLAVE_FORCE: c_ulong = 0x0703; // Set slave address, even if it's already in use by a driver
const REQ_TENBIT: c_ulong = 0x0704; // Use 10-bit slave addresses
const REQ_FUNCS: c_ulong = 0x0705; // Read I2C bus capabilities
const REQ_RDWR: c_ulong = 0x0707; // Combined read/write transfer with a single STOP
const REQ_PEC: c_ulong = 0x0708; // SMBus: Use Packet Error Checking
const REQ_SMBUS: c_ulong = 0x0720; // SMBus: Transfer

// TODO: Check if 10-bit addresses are supported by i2cdev and the underlying drivers

pub unsafe fn set_slave_address(fd: c_int, value: i32) -> Result<i32> {
    parse_retval(ioctl(fd, REQ_SLAVE, &value))
}

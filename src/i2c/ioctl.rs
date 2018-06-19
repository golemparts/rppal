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

// Based on i2c.h, i2c-dev.h and the documentation at https://www.kernel.org/doc/Documentation/i2c
// and http://smbus.org/specs/SMBus_3_1_20180319.pdf

// Capabilities returned by REQ_FUNCS
const FUNC_I2C: c_ulong = 0x01;
const FUNC_10BIT_ADDR: c_ulong = 0x02;
const FUNC_PROTOCOL_MANGLING: c_ulong = 0x04;
const FUNC_SMBUS_PEC: c_ulong = 0x08;
const FUNC_NOSTART: c_ulong = 0x10;
const FUNC_SLAVE: c_ulong = 0x20;
const FUNC_SMBUS_BLOCK_PROC_CALL: c_ulong = 0x8000;
const FUNC_SMBUS_QUICK: c_ulong = 0x01_0000;
const FUNC_SMBUS_READ_BYTE: c_ulong = 0x02_0000;
const FUNC_SMBUS_WRITE_BYTE: c_ulong = 0x04_0000;
const FUNC_SMBUS_READ_BYTE_DATA: c_ulong = 0x08_0000;
const FUNC_SMBUS_WRITE_BYTE_DATA: c_ulong = 0x10_0000;
const FUNC_SMBUS_READ_WORD_DATA: c_ulong = 0x20_0000;
const FUNC_SMBUS_WRITE_WORD_DATA: c_ulong = 0x40_0000;
const FUNC_SMBUS_PROC_CALL: c_ulong = 0x80_0000;
const FUNC_SMBUS_READ_BLOCK_DATA: c_ulong = 0x0100_0000;
const FUNC_SMBUS_WRITE_BLOCK_DATA: c_ulong = 0x0200_0000;
const FUNC_SMBUS_READ_I2C_BLOCK: c_ulong = 0x0400_0000;
const FUNC_SMBUS_WRITE_I2C_BLOCK: c_ulong = 0x0800_0000;
const FUNC_SMBUS_HOST_NOTIFY: c_ulong = 0x1000_0000;

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Capabilities {
    funcs: c_ulong,
}

impl Capabilities {
    fn new(funcs: c_ulong) -> Capabilities {
        Capabilities { funcs }
    }

    pub fn i2c(&self) -> bool {
        (self.funcs & FUNC_I2C) > 0
    }

    pub fn slave(&self) -> bool {
        (self.funcs & FUNC_SLAVE) > 0
    }

    pub fn addr_10bit(&self) -> bool {
        (self.funcs & FUNC_10BIT_ADDR) > 0
    }

    pub fn i2c_block_read(&self) -> bool {
        (self.funcs & FUNC_SMBUS_READ_I2C_BLOCK) > 0
    }

    pub fn i2c_block_write(&self) -> bool {
        (self.funcs & FUNC_SMBUS_WRITE_I2C_BLOCK) > 0
    }

    pub fn protocol_mangling(&self) -> bool {
        (self.funcs & FUNC_PROTOCOL_MANGLING) > 0
    }

    pub fn nostart(&self) -> bool {
        (self.funcs & FUNC_NOSTART) > 0
    }

    pub fn smbus_quick_command(&self) -> bool {
        (self.funcs & FUNC_SMBUS_QUICK) > 0
    }

    pub fn smbus_receive_byte(&self) -> bool {
        (self.funcs & FUNC_SMBUS_READ_BYTE) > 0
    }

    pub fn smbus_send_byte(&self) -> bool {
        (self.funcs & FUNC_SMBUS_WRITE_BYTE) > 0
    }

    pub fn smbus_read_byte(&self) -> bool {
        (self.funcs & FUNC_SMBUS_READ_BYTE_DATA) > 0
    }

    pub fn smbus_write_byte(&self) -> bool {
        (self.funcs & FUNC_SMBUS_WRITE_BYTE_DATA) > 0
    }

    pub fn smbus_read_word(&self) -> bool {
        (self.funcs & FUNC_SMBUS_READ_WORD_DATA) > 0
    }

    pub fn smbus_write_word(&self) -> bool {
        (self.funcs & FUNC_SMBUS_WRITE_WORD_DATA) > 0
    }

    pub fn smbus_process_call(&self) -> bool {
        (self.funcs & FUNC_SMBUS_PROC_CALL) > 0
    }

    pub fn smbus_block_read(&self) -> bool {
        (self.funcs & FUNC_SMBUS_READ_BLOCK_DATA) > 0
    }

    pub fn smbus_block_write(&self) -> bool {
        (self.funcs & FUNC_SMBUS_WRITE_BLOCK_DATA) > 0
    }

    pub fn smbus_block_process_call(&self) -> bool {
        (self.funcs & FUNC_SMBUS_BLOCK_PROC_CALL) > 0
    }

    pub fn smbus_pec(&self) -> bool {
        (self.funcs & FUNC_SMBUS_PEC) > 0
    }

    pub fn smbus_host_notify(&self) -> bool {
        (self.funcs & FUNC_SMBUS_HOST_NOTIFY) > 0
    }
}

// ioctl() requests supported by i2cdev
const REQ_RETRIES: c_ulong = 0x0701; // How many retries when waiting for an ACK
const REQ_TIMEOUT: c_ulong = 0x0702; // Timeout in 10ms units
const REQ_SLAVE: c_ulong = 0x0706; // Set slave address
const REQ_SLAVE_FORCE: c_ulong = 0x0703; // Set slave address, even if it's already in use by a driver
const REQ_TENBIT: c_ulong = 0x0704; // Use 10-bit slave addresses
const REQ_FUNCS: c_ulong = 0x0705; // Read I2C bus capabilities
const REQ_RDWR: c_ulong = 0x0707; // Combined read/write transfer with a single STOP
const REQ_PEC: c_ulong = 0x0708; // SMBus: Use Packet Error Checking
const REQ_SMBUS: c_ulong = 0x0720; // SMBus: Transfer

const SMBUS_BLOCK_MAX: usize = 32; // Maximum bytes per block transfer

// SMBus read or write request
#[derive(Debug, PartialEq, Copy, Clone)]
enum SmbusReadWrite {
    Read = 1,
    Write = 0,
}

// Size/Type identifiers for the data contained in SmbusBuffer
#[derive(Debug, PartialEq, Copy, Clone)]
enum SmbusSize {
    ByteData = 2,
    WordData = 3,
}

// Holds data transferred by REQ_SMBUS requests. Data can either consist of a
// single byte, a 16-bit word, or a block, where the first byte contains the length,
// followed by up to 32 bytes of data, with the final byte used as padding.
#[derive(Copy, Clone)]
#[repr(C)]
struct SmbusBuffer {
    data: [u8; SMBUS_BLOCK_MAX + 2],
}

impl SmbusBuffer {
    pub fn new() -> SmbusBuffer {
        SmbusBuffer {
            data: [0u8; SMBUS_BLOCK_MAX + 2],
        }
    }

    pub fn with_byte(value: u8) -> SmbusBuffer {
        let mut buffer = SmbusBuffer {
            data: [0u8; SMBUS_BLOCK_MAX + 2],
        };

        buffer.data[0] = value;

        buffer
    }

    pub fn with_word(value: u16) -> SmbusBuffer {
        let mut buffer = SmbusBuffer {
            data: [0u8; SMBUS_BLOCK_MAX + 2],
        };

        // Low byte is sent first (SMBus 3.1 spec @ 6.5.4)
        buffer.data[0] = (value & 0xFF) as u8;
        buffer.data[1] = (value >> 8) as u8;

        buffer
    }
}

// Specifies SMBus request parameters
#[repr(C)]
struct SmbusRequest<'a> {
    read_write: u8,
    command: u8,
    size: u32,
    data: &'a mut SmbusBuffer,
}

unsafe fn smbus_request(
    fd: c_int,
    read_write: SmbusReadWrite,
    command: u8,
    size: SmbusSize,
    data: &mut SmbusBuffer,
) -> Result<i32> {
    let mut request = SmbusRequest {
        read_write: read_write as u8,
        command,
        size: size as u32,
        data,
    };

    parse_retval(ioctl(fd, REQ_SMBUS, &mut request))
}

pub unsafe fn smbus_read_byte(fd: c_int, command: u8) -> Result<u8> {
    let mut buffer = SmbusBuffer::new();
    smbus_request(
        fd,
        SmbusReadWrite::Read,
        command,
        SmbusSize::ByteData,
        &mut buffer,
    )?;

    Ok(buffer.data[0])
}

pub unsafe fn smbus_read_word(fd: c_int, command: u8) -> Result<u16> {
    let mut buffer = SmbusBuffer::new();
    smbus_request(
        fd,
        SmbusReadWrite::Read,
        command,
        SmbusSize::WordData,
        &mut buffer,
    )?;

    // Low byte is received first (SMBus 3.1 spec @ 6.5.5)
    Ok(u16::from(buffer.data[0]) | (u16::from(buffer.data[1]) << 8))
}

pub unsafe fn smbus_write_byte(fd: c_int, command: u8, value: u8) -> Result<i32> {
    let mut buffer = SmbusBuffer::with_byte(value);
    smbus_request(
        fd,
        SmbusReadWrite::Write,
        command,
        SmbusSize::ByteData,
        &mut buffer,
    )
}

pub unsafe fn smbus_write_word(fd: c_int, command: u8, value: u16) -> Result<i32> {
    let mut buffer = SmbusBuffer::with_word(value);
    smbus_request(
        fd,
        SmbusReadWrite::Write,
        command,
        SmbusSize::WordData,
        &mut buffer,
    )
}

// TODO: Check if 10-bit addresses are supported by i2cdev and the underlying drivers

// All ioctl commands take an unsigned long parameter, except for
// REQ_FUNCS, REQ_RDWR (pointer to ic2_rdwr_ioctl_data) and REQ_SMBUS

pub unsafe fn set_slave_address(fd: c_int, value: c_ulong) -> Result<i32> {
    parse_retval(ioctl(fd, REQ_SLAVE, value))
}

pub unsafe fn set_addr_10bit(fd: c_int, value: c_ulong) -> Result<i32> {
    parse_retval(ioctl(fd, REQ_TENBIT, value))
}

pub unsafe fn set_pec(fd: c_int, value: c_ulong) -> Result<i32> {
    parse_retval(ioctl(fd, REQ_PEC, value))
}

pub unsafe fn funcs(fd: c_int) -> Result<Capabilities> {
    let mut funcs: c_ulong = 0;

    parse_retval(ioctl(fd, REQ_FUNCS, &mut funcs))?;

    Ok(Capabilities::new(funcs))
}

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

use std::fmt;
use std::io;
use std::ptr;
use std::result;

use libc::{self, c_int, c_ulong, ioctl};

#[cfg(target_env = "gnu")]
type IoctlLong = libc::c_ulong;
#[cfg(target_env = "musl")]
type IoctlLong = c_int;

pub type Result<T> = result::Result<T, io::Error>;

// Based on i2c.h, i2c-dev.c, i2c-dev.h and the documentation at https://www.kernel.org/doc/Documentation/i2c
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

/// Lists the features supported by the underlying drivers.
#[derive(PartialEq, Copy, Clone)]
pub struct Capabilities {
    funcs: c_ulong,
}

impl Capabilities {
    /// Constructs a new `Capabilities`.
    ///
    /// `Capabilities` indicates which I2C features and SMBus protocols
    /// are supported by the underlying drivers.
    fn new(funcs: c_ulong) -> Capabilities {
        Capabilities { funcs }
    }

    pub(crate) fn i2c(self) -> bool {
        (self.funcs & FUNC_I2C) > 0
    }

    pub(crate) fn slave(self) -> bool {
        (self.funcs & FUNC_SLAVE) > 0
    }

    /// Indicates whether 10-bit addresses are supported.
    pub fn addr_10bit(self) -> bool {
        (self.funcs & FUNC_10BIT_ADDR) > 0
    }

    /// Indicates whether I2C Block Read is supported.
    pub fn i2c_block_read(self) -> bool {
        (self.funcs & FUNC_SMBUS_READ_I2C_BLOCK) > 0
    }

    /// Indicates whether I2C Block Write is supported.
    pub fn i2c_block_write(self) -> bool {
        (self.funcs & FUNC_SMBUS_WRITE_I2C_BLOCK) > 0
    }

    /// Indicates whether protocol mangling is supported.
    pub(crate) fn protocol_mangling(self) -> bool {
        (self.funcs & FUNC_PROTOCOL_MANGLING) > 0
    }

    /// Indicates whether the NOSTART flag is supported.
    pub(crate) fn nostart(self) -> bool {
        (self.funcs & FUNC_NOSTART) > 0
    }

    /// Indicates whether SMBus Quick Command is supported.
    pub fn smbus_quick_command(self) -> bool {
        (self.funcs & FUNC_SMBUS_QUICK) > 0
    }

    /// Indicates whether SMBus Receive Byte is supported.
    pub fn smbus_receive_byte(self) -> bool {
        (self.funcs & FUNC_SMBUS_READ_BYTE) > 0
    }

    /// Indicates whether SMBus Send Byte is supported.
    pub fn smbus_send_byte(self) -> bool {
        (self.funcs & FUNC_SMBUS_WRITE_BYTE) > 0
    }

    /// Indicates whether SMBus Read Byte is supported.
    pub fn smbus_read_byte(self) -> bool {
        (self.funcs & FUNC_SMBUS_READ_BYTE_DATA) > 0
    }

    /// Indicates whether SMBus Write Byte is supported.
    pub fn smbus_write_byte(self) -> bool {
        (self.funcs & FUNC_SMBUS_WRITE_BYTE_DATA) > 0
    }

    /// Indicates whether SMBus Read Word is supported.
    pub fn smbus_read_word(self) -> bool {
        (self.funcs & FUNC_SMBUS_READ_WORD_DATA) > 0
    }

    /// Indicates whether SMBus Write Word is supported.
    pub fn smbus_write_word(self) -> bool {
        (self.funcs & FUNC_SMBUS_WRITE_WORD_DATA) > 0
    }

    /// Indicates whether SMBus Process Call is supported.
    pub fn smbus_process_call(self) -> bool {
        (self.funcs & FUNC_SMBUS_PROC_CALL) > 0
    }

    /// Indicates whether SMBus Block Read is supported.
    pub fn smbus_block_read(self) -> bool {
        (self.funcs & FUNC_SMBUS_READ_BLOCK_DATA) > 0
    }

    /// Indicates whether SMBus Block Write is supported.
    pub fn smbus_block_write(self) -> bool {
        (self.funcs & FUNC_SMBUS_WRITE_BLOCK_DATA) > 0
    }

    /// Indicates whether SMBus Block Process Call is supported.
    pub fn smbus_block_process_call(self) -> bool {
        (self.funcs & FUNC_SMBUS_BLOCK_PROC_CALL) > 0
    }

    /// Indicates whether SMBus Packet Error Checking is supported.
    pub fn smbus_pec(self) -> bool {
        (self.funcs & FUNC_SMBUS_PEC) > 0
    }

    /// Indicates whether SMBus Host Notify is supported.
    pub fn smbus_host_notify(self) -> bool {
        (self.funcs & FUNC_SMBUS_HOST_NOTIFY) > 0
    }
}

impl fmt::Debug for Capabilities {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Capabilities")
            .field("addr_10bit", &self.addr_10bit())
            .field("i2c_block_read", &self.i2c_block_read())
            .field("i2c_block_write", &self.i2c_block_write())
            .field("smbus_quick_command", &self.smbus_quick_command())
            .field("smbus_receive_byte", &self.smbus_receive_byte())
            .field("smbus_send_byte", &self.smbus_send_byte())
            .field("smbus_read_byte", &self.smbus_read_byte())
            .field("smbus_write_byte", &self.smbus_write_byte())
            .field("smbus_read_word", &self.smbus_read_word())
            .field("smbus_write_word", &self.smbus_write_word())
            .field("smbus_process_call", &self.smbus_process_call())
            .field("smbus_block_read", &self.smbus_block_read())
            .field("smbus_block_write", &self.smbus_block_write())
            .field("smbus_block_process_call", &self.smbus_block_process_call())
            .field("smbus_pec", &self.smbus_pec())
            .field("smbus_host_notify", &self.smbus_host_notify())
            .finish()
    }
}

// ioctl() requests supported by i2cdev
const REQ_RETRIES: IoctlLong = 0x0701; // How many retries when waiting for an ACK
const REQ_TIMEOUT: IoctlLong = 0x0702; // Timeout in 10ms units
const REQ_SLAVE: IoctlLong = 0x0706; // Set slave address
const REQ_SLAVE_FORCE: IoctlLong = 0x0703; // Set slave address, even if it's already in use by a driver
const REQ_TENBIT: IoctlLong = 0x0704; // Use 10-bit slave addresses
const REQ_FUNCS: IoctlLong = 0x0705; // Read I2C bus capabilities
const REQ_RDWR: IoctlLong = 0x0707; // Combined read/write transfer with a single STOP
const REQ_PEC: IoctlLong = 0x0708; // SMBus: Use Packet Error Checking
const REQ_SMBUS: IoctlLong = 0x0720; // SMBus: Transfer data

// NOTE: REQ_RETRIES - Supported in i2cdev, but not used in the underlying drivers
// NOTE: REQ_RDWR - Only a single read operation is supported as the final message (see i2c-bcm2835.c)

const RDWR_FLAG_RD: u16 = 0x0001; // Read operation
const RDWR_FLAG_TEN: u16 = 0x0010; // 10-bit slave address

const RDWR_MSG_MAX: usize = 42; // Maximum messages per RDWR operation
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
    Quick = 0,
    Byte = 1,
    ByteData = 2,
    WordData = 3,
    ProcCall = 4,
    BlockData = 5,
    I2cBlockData = 8,
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

    pub fn with_buffer(value: &[u8]) -> SmbusBuffer {
        let mut buffer = SmbusBuffer {
            data: [0u8; SMBUS_BLOCK_MAX + 2],
        };

        buffer.data[0] = if value.len() > SMBUS_BLOCK_MAX {
            buffer.data[1..=SMBUS_BLOCK_MAX].copy_from_slice(&value[..SMBUS_BLOCK_MAX]);
            SMBUS_BLOCK_MAX as u8
        } else {
            buffer.data[1..=value.len()].copy_from_slice(&value);
            value.len() as u8
        };

        buffer
    }
}

// Specifies SMBus request parameters
#[repr(C)]
struct SmbusRequest {
    // Read (1) or write (0) request.
    read_write: u8,
    // User-specified 8-bit command value.
    command: u8,
    // Request type identifier.
    size: u32,
    // Pointer to buffer, or 0.
    data: *mut SmbusBuffer,
}

fn smbus_request(
    fd: c_int,
    read_write: SmbusReadWrite,
    command: u8,
    size: SmbusSize,
    data: Option<&mut SmbusBuffer>,
) -> Result<()> {
    let mut request = SmbusRequest {
        read_write: read_write as u8,
        command,
        size: size as u32,
        data: if let Some(buffer) = data {
            buffer
        } else {
            ptr::null_mut()
        },
    };

    parse_retval!(unsafe { ioctl(fd, REQ_SMBUS, &mut request) })?;

    Ok(())
}

pub fn smbus_quick_command(fd: c_int, value: bool) -> Result<()> {
    // Quick Command uses the read_write field, instead of the data buffer
    smbus_request(
        fd,
        if value {
            SmbusReadWrite::Read
        } else {
            SmbusReadWrite::Write
        },
        0,
        SmbusSize::Quick,
        None,
    )
}

pub fn smbus_receive_byte(fd: c_int) -> Result<u8> {
    let mut buffer = SmbusBuffer::new();
    smbus_request(
        fd,
        SmbusReadWrite::Read,
        0,
        SmbusSize::Byte,
        Some(&mut buffer),
    )?;

    Ok(buffer.data[0])
}

pub fn smbus_send_byte(fd: c_int, value: u8) -> Result<()> {
    // Send Byte uses the command field, instead of the data buffer
    smbus_request(fd, SmbusReadWrite::Write, value, SmbusSize::Byte, None)
}

pub fn smbus_read_byte(fd: c_int, command: u8) -> Result<u8> {
    let mut buffer = SmbusBuffer::new();
    smbus_request(
        fd,
        SmbusReadWrite::Read,
        command,
        SmbusSize::ByteData,
        Some(&mut buffer),
    )?;

    Ok(buffer.data[0])
}

pub fn smbus_read_word(fd: c_int, command: u8) -> Result<u16> {
    let mut buffer = SmbusBuffer::new();
    smbus_request(
        fd,
        SmbusReadWrite::Read,
        command,
        SmbusSize::WordData,
        Some(&mut buffer),
    )?;

    // Low byte is received first (SMBus 3.1 spec @ 6.5.5)
    Ok(u16::from(buffer.data[0]) | (u16::from(buffer.data[1]) << 8))
}

pub fn smbus_write_byte(fd: c_int, command: u8, value: u8) -> Result<()> {
    let mut buffer = SmbusBuffer::with_byte(value);
    smbus_request(
        fd,
        SmbusReadWrite::Write,
        command,
        SmbusSize::ByteData,
        Some(&mut buffer),
    )
}

pub fn smbus_write_word(fd: c_int, command: u8, value: u16) -> Result<()> {
    let mut buffer = SmbusBuffer::with_word(value);
    smbus_request(
        fd,
        SmbusReadWrite::Write,
        command,
        SmbusSize::WordData,
        Some(&mut buffer),
    )
}

pub fn smbus_process_call(fd: c_int, command: u8, value: u16) -> Result<u16> {
    let mut buffer = SmbusBuffer::with_word(value);
    smbus_request(
        fd,
        SmbusReadWrite::Write,
        command,
        SmbusSize::ProcCall,
        Some(&mut buffer),
    )?;

    // Low byte is received first (SMBus 3.1 spec @ 6.5.6)
    Ok(u16::from(buffer.data[0]) | (u16::from(buffer.data[1]) << 8))
}

pub fn smbus_block_read(fd: c_int, command: u8, value: &mut [u8]) -> Result<usize> {
    let mut buffer = SmbusBuffer::new();
    smbus_request(
        fd,
        SmbusReadWrite::Read,
        command,
        SmbusSize::BlockData,
        Some(&mut buffer),
    )?;

    // Verify the length in case we're receiving corrupted data
    let incoming_length = if buffer.data[0] as usize > SMBUS_BLOCK_MAX {
        SMBUS_BLOCK_MAX
    } else {
        buffer.data[0] as usize
    };

    // Make sure the incoming data fits in the value buffer
    let value_length = value.len();
    if incoming_length > value_length {
        value.copy_from_slice(&buffer.data[1..=value_length]);
    } else {
        value[..incoming_length].copy_from_slice(&buffer.data[1..=incoming_length]);
    }

    Ok(incoming_length)
}

pub fn smbus_block_write(fd: c_int, command: u8, value: &[u8]) -> Result<()> {
    let mut buffer = SmbusBuffer::with_buffer(value);
    smbus_request(
        fd,
        SmbusReadWrite::Write,
        command,
        SmbusSize::BlockData,
        Some(&mut buffer),
    )
}

pub fn i2c_block_read(fd: c_int, command: u8, value: &mut [u8]) -> Result<()> {
    let mut buffer = SmbusBuffer::new();
    buffer.data[0] = if value.len() > SMBUS_BLOCK_MAX {
        SMBUS_BLOCK_MAX as u8
    } else {
        value.len() as u8
    };

    smbus_request(
        fd,
        SmbusReadWrite::Read,
        command,
        SmbusSize::I2cBlockData,
        Some(&mut buffer),
    )?;

    value[..buffer.data[0] as usize].copy_from_slice(&buffer.data[1..=buffer.data[0] as usize]);

    Ok(())
}

pub fn i2c_block_write(fd: c_int, command: u8, value: &[u8]) -> Result<()> {
    let mut buffer = SmbusBuffer::with_buffer(value);
    smbus_request(
        fd,
        SmbusReadWrite::Write,
        command,
        SmbusSize::I2cBlockData,
        Some(&mut buffer),
    )
}

// Specifies RDWR segment parameters
#[repr(C)]
#[derive(Debug, PartialEq, Copy, Clone)]
struct RdwrSegment {
    // Slave address
    addr: u16,
    // Segment flags
    flags: u16,
    // Buffer length
    len: u16,
    // Pointer to buffer
    data: usize,
}

// Specifies RWDR request parameters
#[repr(C)]
#[derive(Debug, PartialEq, Copy, Clone)]
struct RdwrRequest {
    // Pointer to an array of segments
    segments: *mut [RdwrSegment],
    // Number of segments
    nmsgs: u32,
}

pub fn i2c_write_read(
    fd: c_int,
    address: u16,
    addr_10bit: bool,
    write_buffer: &[u8],
    read_buffer: &mut [u8],
) -> Result<()> {
    // 0 length buffers may cause issues
    if write_buffer.is_empty() || read_buffer.is_empty() {
        return Ok(());
    }

    let segment_write = RdwrSegment {
        addr: address,
        flags: if addr_10bit { RDWR_FLAG_TEN } else { 0 },
        len: write_buffer.len() as u16,
        data: write_buffer.as_ptr() as usize,
    };

    let segment_read = RdwrSegment {
        addr: address,
        flags: if addr_10bit {
            RDWR_FLAG_RD | RDWR_FLAG_TEN
        } else {
            RDWR_FLAG_RD
        },
        len: read_buffer.len() as u16,
        data: read_buffer.as_mut_ptr() as usize,
    };

    let mut segments: [RdwrSegment; 2] = [segment_write, segment_read];
    let mut request = RdwrRequest {
        segments: &mut segments,
        nmsgs: 2,
    };

    parse_retval!(unsafe { ioctl(fd, REQ_RDWR, &mut request) })?;

    Ok(())
}

pub fn set_slave_address(fd: c_int, value: c_ulong) -> Result<()> {
    parse_retval!(unsafe { ioctl(fd, REQ_SLAVE, value) })?;

    Ok(())
}

pub fn set_addr_10bit(fd: c_int, value: c_ulong) -> Result<()> {
    parse_retval!(unsafe { ioctl(fd, REQ_TENBIT, value) })?;

    Ok(())
}

pub fn set_pec(fd: c_int, value: c_ulong) -> Result<()> {
    parse_retval!(unsafe { ioctl(fd, REQ_PEC, value) })?;

    Ok(())
}

pub fn set_timeout(fd: c_int, value: c_ulong) -> Result<()> {
    // Timeout is specified in units of 10ms
    let timeout: c_ulong = if value > 0 && value < 10 {
        1
    } else {
        value / 10
    };

    parse_retval!(unsafe { ioctl(fd, REQ_TIMEOUT, timeout) })?;

    Ok(())
}

pub fn set_retries(fd: c_int, value: c_ulong) -> Result<()> {
    // Number of retries on arbitration loss
    parse_retval!(unsafe { ioctl(fd, REQ_RETRIES, value) })?;

    Ok(())
}

pub fn funcs(fd: c_int) -> Result<Capabilities> {
    let mut funcs: c_ulong = 0;

    parse_retval!(unsafe { ioctl(fd, REQ_FUNCS, &mut funcs) })?;

    Ok(Capabilities::new(funcs))
}

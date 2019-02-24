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

#![allow(dead_code)]

use libc::{self, c_int, ioctl};
use std::io;
use std::mem;
use std::result;

use super::segment::Segment;

#[cfg(target_env = "gnu")]
type IoctlLong = libc::c_ulong;
#[cfg(target_env = "musl")]
type IoctlLong = c_int;

pub type Result<T> = result::Result<T, io::Error>;

const NRBITS: u8 = 8;
const TYPEBITS: u8 = 8;
const SIZEBITS: u8 = 14;
const DIRBITS: u8 = 2;

const NRSHIFT: u8 = 0;
const TYPESHIFT: u8 = (NRSHIFT + NRBITS);
const SIZESHIFT: u8 = (TYPESHIFT + TYPEBITS);
const DIRSHIFT: u8 = (SIZESHIFT + SIZEBITS);

const NR_MESSAGE: IoctlLong = 0 << NRSHIFT;
const NR_MODE: IoctlLong = 1 << NRSHIFT;
const NR_LSB_FIRST: IoctlLong = 2 << NRSHIFT;
const NR_BITS_PER_WORD: IoctlLong = 3 << NRSHIFT;
const NR_MAX_SPEED_HZ: IoctlLong = 4 << NRSHIFT;
const NR_MODE32: IoctlLong = 5 << NRSHIFT;

const TYPE_SPI: IoctlLong = (b'k' as IoctlLong) << TYPESHIFT;

const SIZE_U8: IoctlLong = (mem::size_of::<u8>() as IoctlLong) << SIZESHIFT;
const SIZE_U32: IoctlLong = (mem::size_of::<u32>() as IoctlLong) << SIZESHIFT;

const DIR_NONE: IoctlLong = 0;
const DIR_WRITE: IoctlLong = 1 << DIRSHIFT;
const DIR_READ: IoctlLong = 2 << DIRSHIFT;

const REQ_RD_MODE: IoctlLong = (DIR_READ | TYPE_SPI | NR_MODE | SIZE_U8);
const REQ_RD_LSB_FIRST: IoctlLong = (DIR_READ | TYPE_SPI | NR_LSB_FIRST | SIZE_U8);
const REQ_RD_BITS_PER_WORD: IoctlLong = (DIR_READ | TYPE_SPI | NR_BITS_PER_WORD | SIZE_U8);
const REQ_RD_MAX_SPEED_HZ: IoctlLong = (DIR_READ | TYPE_SPI | NR_MAX_SPEED_HZ | SIZE_U32);
const REQ_RD_MODE_32: IoctlLong = (DIR_READ | TYPE_SPI | NR_MODE32 | SIZE_U32);

const REQ_WR_MESSAGE: IoctlLong = (DIR_WRITE | TYPE_SPI | NR_MESSAGE);
const REQ_WR_MODE: IoctlLong = (DIR_WRITE | TYPE_SPI | NR_MODE | SIZE_U8);
const REQ_WR_LSB_FIRST: IoctlLong = (DIR_WRITE | TYPE_SPI | NR_LSB_FIRST | SIZE_U8);
const REQ_WR_BITS_PER_WORD: IoctlLong = (DIR_WRITE | TYPE_SPI | NR_BITS_PER_WORD | SIZE_U8);
const REQ_WR_MAX_SPEED_HZ: IoctlLong = (DIR_WRITE | TYPE_SPI | NR_MAX_SPEED_HZ | SIZE_U32);
const REQ_WR_MODE_32: IoctlLong = (DIR_WRITE | TYPE_SPI | NR_MODE32 | SIZE_U32);

pub const MODE_CPHA: u8 = 0x01;
pub const MODE_CPOL: u8 = 0x02;

pub const MODE_0: u8 = 0;
pub const MODE_1: u8 = MODE_CPHA;
pub const MODE_2: u8 = MODE_CPOL;
pub const MODE_3: u8 = MODE_CPOL | MODE_CPHA;

pub const MODE_CS_HIGH: u8 = 0x04; // Set SS to active high
pub const MODE_LSB_FIRST: u8 = 0x08; // Set bit order to LSB first
pub const MODE_3WIRE: u8 = 0x10; // Set bidirectional mode
pub const MODE_LOOP: u8 = 0x20; // Set loopback mode
pub const MODE_NO_CS: u8 = 0x40; // Don't assert SS
pub const MODE_READY: u8 = 0x80; // Slave sends a ready signal
pub const MODE_TX_DUAL: u32 = 0x0100; // Send on 2 outgoing lines
pub const MODE_TX_QUAD: u32 = 0x0200; // Send on 4 outgoing lines
pub const MODE_RX_DUAL: u32 = 0x0400; // Receive on 2 incoming lines
pub const MODE_RX_QUAD: u32 = 0x0800; // Receive on 4 incoming lines

pub fn mode(fd: c_int, value: &mut u8) -> Result<i32> {
    parse_retval!(unsafe { ioctl(fd, REQ_RD_MODE, value) })
}

pub fn set_mode(fd: c_int, value: u8) -> Result<i32> {
    parse_retval!(unsafe { ioctl(fd, REQ_WR_MODE, &value) })
}

pub fn lsb_first(fd: c_int, value: &mut u8) -> Result<i32> {
    parse_retval!(unsafe { ioctl(fd, REQ_RD_LSB_FIRST, value) })
}

pub fn set_lsb_first(fd: c_int, value: u8) -> Result<i32> {
    parse_retval!(unsafe { ioctl(fd, REQ_WR_LSB_FIRST, &value) })
}

pub fn bits_per_word(fd: c_int, value: &mut u8) -> Result<i32> {
    parse_retval!(unsafe { ioctl(fd, REQ_RD_BITS_PER_WORD, value) })
}

pub fn set_bits_per_word(fd: c_int, value: u8) -> Result<i32> {
    parse_retval!(unsafe { ioctl(fd, REQ_WR_BITS_PER_WORD, &value) })
}

pub fn clock_speed(fd: c_int, value: &mut u32) -> Result<i32> {
    parse_retval!(unsafe { ioctl(fd, REQ_RD_MAX_SPEED_HZ, value) })
}

pub fn set_clock_speed(fd: c_int, value: u32) -> Result<i32> {
    parse_retval!(unsafe { ioctl(fd, REQ_WR_MAX_SPEED_HZ, &value) })
}

pub fn mode32(fd: c_int, value: &mut u32) -> Result<i32> {
    parse_retval!(unsafe { ioctl(fd, REQ_RD_MODE_32, value) })
}

pub fn set_mode32(fd: c_int, value: u32) -> Result<i32> {
    parse_retval!(unsafe { ioctl(fd, REQ_WR_MODE_32, &value) })
}

pub fn transfer(fd: c_int, segments: &[Segment<'_, '_>]) -> Result<i32> {
    parse_retval!(unsafe {
        ioctl(
            fd,
            REQ_WR_MESSAGE
                | (((segments.len() * mem::size_of::<Segment<'_, '_>>()) as IoctlLong)
                    << SIZESHIFT),
            segments,
        )
    })
}

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

// NOTE: This is currently experimental, and may be removed at any point.

#![allow(dead_code)]

use libc::{c_int, c_ulong, ioctl};
use std::io;
use std::mem::size_of;
use std::result;

pub type Result<T> = result::Result<T, io::Error>;

fn parse_retval(retval: c_int) -> Result<i32> {
    if retval == -1 {
        Err(io::Error::last_os_error())
    } else {
        Ok(retval)
    }
}

#[repr(C)]
struct GpioChipInfo {
    name: [u8; 32],
    label: [u8; 32],
    lines: u32,
}

#[repr(C)]
struct GpioLineInfo {
    line_offset: u32,
    flags: u32,
    name: [u8; 32],
    consumer: [u8; 32],
}

const GPIOHANDLES_MAX: usize = 64;

#[repr(C)]
struct GpioHandleRequest {
    line_offets: [u32; GPIOHANDLES_MAX],
    flags: u32,
    default_values: [u8; GPIOHANDLES_MAX],
    consumer_label: [u8; 32],
    lines: u32,
    fd: c_int,
}

#[repr(C)]
struct GpioEventRequest {
    line_offset: u32,
    handle_flags: u32,
    event_flags: u32,
    consumer_label: [u8; 32],
    fd: c_int,
}

const NRBITS: u8 = 8;
const TYPEBITS: u8 = 8;
const SIZEBITS: u8 = 14;
const DIRBITS: u8 = 2;

const NRSHIFT: u8 = 0;
const TYPESHIFT: u8 = (NRSHIFT + NRBITS);
const SIZESHIFT: u8 = (TYPESHIFT + TYPEBITS);
const DIRSHIFT: u8 = (SIZESHIFT + SIZEBITS);

const NR_GET_CHIPINFO: c_ulong = 0x01 << NRSHIFT;
const NR_GET_LINEINFO: c_ulong = 0x02 << NRSHIFT;
const NR_GET_LINEHANDLE: c_ulong = 0x03 << NRSHIFT;
const NR_GET_LINEEVENT: c_ulong = 0x04 << NRSHIFT;

const TYPE_GPIO: c_ulong = (0xB4 as c_ulong) << TYPESHIFT;

const SIZE_GPIOCHIPINFO: c_ulong = (size_of::<GpioChipInfo>() as c_ulong) << SIZESHIFT;
const SIZE_GPIOLINEINFO: c_ulong = (size_of::<GpioLineInfo>() as c_ulong) << SIZESHIFT;
const SIZE_GPIOHANDLEREQUEST: c_ulong = (size_of::<GpioHandleRequest>() as c_ulong) << SIZESHIFT;
const SIZE_GPIOEVENTREQUEST: c_ulong = (size_of::<GpioEventRequest>() as c_ulong) << SIZESHIFT;

const DIR_NONE: c_ulong = 0;
const DIR_WRITE: c_ulong = 1 << DIRSHIFT;
const DIR_READ: c_ulong = 2 << DIRSHIFT;

const REQ_GET_CHIPINFO: c_ulong = DIR_READ | TYPE_GPIO | NR_GET_CHIPINFO | SIZE_GPIOCHIPINFO;
const REQ_GET_LINEINFO: c_ulong = DIR_READ | DIR_WRITE | TYPE_GPIO | NR_GET_LINEINFO | SIZE_GPIOLINEINFO;
const REQ_GET_LINEHANDLE: c_ulong = DIR_READ | DIR_WRITE | TYPE_GPIO | NR_GET_LINEHANDLE | SIZE_GPIOHANDLEREQUEST;
const REQ_GET_LINEEVENT: c_ulong = DIR_READ | DIR_WRITE | TYPE_GPIO | NR_GET_LINEEVENT | SIZE_GPIOEVENTREQUEST;

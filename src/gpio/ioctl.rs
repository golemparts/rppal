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
struct ChipInfo {
    name: [u8; 32],
    label: [u8; 32],
    lines: u32,
}

const LINE_FLAG_KERNEL: u32 = 1;
const LINE_FLAG_IS_OUT: u32 = 1 << 1;
const LINE_FLAG_ACTIVE_LOW: u32 = 1 << 2;
const LINE_FLAG_OPEN_DRAIN: u32 = 1 << 3;
const LINE_FLAG_OPEN_SOURCE: u32 = 1 << 4;

#[repr(C)]
struct LineInfo {
    line_offset: u32,
    flags: u32,
    name: [u8; 32],
    consumer: [u8; 32],
}

const HANDLES_MAX: usize = 64;
const HANDLE_FLAG_INPUT: u32 = 1;
const HANDLE_FLAG_OUTPUT: u32 = 1 << 1;
const HANDLE_FLAG_ACTIVE_LOW: u32 = 1 << 2;
const HANDLE_FLAG_OPEN_DRAIN: u32 = 1 << 3;
const HANDLE_FLAG_OPEN_SOURCE: u32 = 1 << 4;

#[repr(C)]
struct HandleRequest {
    line_offets: [u32; HANDLES_MAX],
    flags: u32,
    default_values: [u8; HANDLES_MAX],
    consumer_label: [u8; 32],
    lines: u32,
    fd: c_int,
}

#[repr(C)]
struct HandleData {
    values: [u8; HANDLES_MAX],
}

const EVENT_FLAG_RISING_EDGE: u32 = 1;
const EVENT_FLAG_FALLING_EDGE: u32 = 1 << 1;
const EVENT_FLAG_BOTH_EDGES: u32 = EVENT_FLAG_RISING_EDGE | EVENT_FLAG_FALLING_EDGE;

#[repr(C)]
struct EventRequest {
    line_offset: u32,
    handle_flags: u32,
    event_flags: u32,
    consumer_label: [u8; 32],
    fd: c_int,
}

const EVENT_TYPE_RISING_EDGE: u32 = 0x01;
const EVENT_TYPE_FALLING_EDGE: u32 = 0x02;

#[repr(C)]
struct EventData {
    timestamp: u64,
    id: u32,
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
const NR_GET_LINE_VALUES: c_ulong = 0x08 << NRSHIFT;
const NR_SET_LINE_VALUES: c_ulong = 0x09 << NRSHIFT;

const TYPE_GPIO: c_ulong = (0xB4 as c_ulong) << TYPESHIFT;

const SIZE_CHIPINFO: c_ulong = (size_of::<ChipInfo>() as c_ulong) << SIZESHIFT;
const SIZE_LINEINFO: c_ulong = (size_of::<LineInfo>() as c_ulong) << SIZESHIFT;
const SIZE_HANDLEREQUEST: c_ulong = (size_of::<HandleRequest>() as c_ulong) << SIZESHIFT;
const SIZE_EVENTREQUEST: c_ulong = (size_of::<EventRequest>() as c_ulong) << SIZESHIFT;
const SIZE_HANDLEDATA: c_ulong = (size_of::<HandleData>() as c_ulong) << SIZESHIFT;

const DIR_NONE: c_ulong = 0;
const DIR_WRITE: c_ulong = 1 << DIRSHIFT;
const DIR_READ: c_ulong = 2 << DIRSHIFT;
const DIR_READ_WRITE: c_ulong = DIR_READ | DIR_WRITE;

const REQ_GET_CHIPINFO: c_ulong = DIR_READ | TYPE_GPIO | NR_GET_CHIPINFO | SIZE_CHIPINFO;
const REQ_GET_LINEINFO: c_ulong = DIR_READ_WRITE | TYPE_GPIO | NR_GET_LINEINFO | SIZE_LINEINFO;
const REQ_GET_LINEHANDLE: c_ulong =
    DIR_READ_WRITE | TYPE_GPIO | NR_GET_LINEHANDLE | SIZE_HANDLEREQUEST;
const REQ_GET_LINEEVENT: c_ulong =
    DIR_READ_WRITE | TYPE_GPIO | NR_GET_LINEEVENT | SIZE_EVENTREQUEST;
const REQ_GET_LINE_VALUES: c_ulong =
    DIR_READ_WRITE | TYPE_GPIO | NR_GET_LINE_VALUES | SIZE_HANDLEDATA;
const REQ_SET_LINE_VALUES: c_ulong =
    DIR_READ_WRITE | TYPE_GPIO | NR_SET_LINE_VALUES | SIZE_HANDLEDATA;

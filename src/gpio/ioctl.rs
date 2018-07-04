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

use libc::{c_int, c_ulong, c_void, ioctl, read};
use std::fs::{File, OpenOptions};
use std::io;
use std::mem::size_of;
use std::os::unix::io::AsRawFd;

use gpio::epoll::{epoll_event, Epoll, EPOLLIN, EPOLLPRI};
use gpio::{Error, Level, Result, Trigger};

fn parse_retval(retval: c_int) -> Result<i32> {
    if retval == -1 {
        Err(Error::Io(io::Error::last_os_error()))
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

impl ChipInfo {
    pub fn new() -> ChipInfo {
        ChipInfo {
            name: [0u8; 32],
            label: [0u8; 32],
            lines: 0,
        }
    }
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

impl LineInfo {
    pub fn new() -> LineInfo {
        LineInfo {
            line_offset: 0,
            flags: 0,
            name: [0u8; 32],
            consumer: [0u8; 32],
        }
    }
}

const HANDLES_MAX: usize = 64;
const HANDLE_FLAG_INPUT: u32 = 1;
const HANDLE_FLAG_OUTPUT: u32 = 1 << 1;
const HANDLE_FLAG_ACTIVE_LOW: u32 = 1 << 2;
const HANDLE_FLAG_OPEN_DRAIN: u32 = 1 << 3;
const HANDLE_FLAG_OPEN_SOURCE: u32 = 1 << 4;

#[repr(C)]
struct HandleRequest {
    line_offsets: [u32; HANDLES_MAX],
    flags: u32,
    default_values: [u8; HANDLES_MAX],
    consumer_label: [u8; 32],
    lines: u32,
    fd: c_int,
}

impl HandleRequest {
    pub fn new() -> HandleRequest {
        HandleRequest {
            line_offsets: [0u32; HANDLES_MAX],
            flags: 0,
            default_values: [0u8; HANDLES_MAX],
            consumer_label: [0u8; 32],
            lines: 0,
            fd: 0,
        }
    }
}

#[repr(C)]
struct HandleData {
    values: [u8; HANDLES_MAX],
}

impl HandleData {
    pub fn new() -> HandleData {
        HandleData {
            values: [0u8; HANDLES_MAX],
        }
    }
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

impl EventRequest {
    pub fn new(pin: u8, handle_flags: u32, trigger: Trigger) -> EventRequest {
        EventRequest {
            line_offset: u32::from(pin),
            handle_flags,
            event_flags: trigger as u32,
            consumer_label: [0u8; 32],
            fd: 0,
        }
    }
}

const EVENT_TYPE_RISING_EDGE: u32 = 0x01;
const EVENT_TYPE_FALLING_EDGE: u32 = 0x02;

#[repr(C)]
#[derive(Copy, Clone)]
struct EventData {
    timestamp: u64,
    id: u32,
}

impl EventData {
    pub fn new() -> EventData {
        EventData {
            timestamp: 0,
            id: 0,
        }
    }

    pub fn from_fd(fd: i32) -> Result<EventData> {
        let mut event_data = EventData::new();

        unsafe {
            parse_retval(read(
                fd,
                &mut event_data as *mut EventData as *mut c_void,
                size_of::<EventData>(),
            ) as i32)?;
        }

        Ok(event_data)
    }
}

const NRBITS: u8 = 8;
const TYPEBITS: u8 = 8;
const SIZEBITS: u8 = 14;
const DIRBITS: u8 = 2;

const NRSHIFT: u8 = 0;
const TYPESHIFT: u8 = (NRSHIFT + NRBITS);
const SIZESHIFT: u8 = (TYPESHIFT + TYPEBITS);
const DIRSHIFT: u8 = (SIZESHIFT + SIZEBITS);

const NR_GET_CHIP_INFO: c_ulong = 0x01 << NRSHIFT;
const NR_GET_LINE_INFO: c_ulong = 0x02 << NRSHIFT;
const NR_GET_LINE_HANDLE: c_ulong = 0x03 << NRSHIFT;
const NR_GET_LINE_EVENT: c_ulong = 0x04 << NRSHIFT;
const NR_GET_LINE_VALUES: c_ulong = 0x08 << NRSHIFT;
const NR_SET_LINE_VALUES: c_ulong = 0x09 << NRSHIFT;

const TYPE_GPIO: c_ulong = (0xB4 as c_ulong) << TYPESHIFT;

const SIZE_CHIP_INFO: c_ulong = (size_of::<ChipInfo>() as c_ulong) << SIZESHIFT;
const SIZE_LINE_INFO: c_ulong = (size_of::<LineInfo>() as c_ulong) << SIZESHIFT;
const SIZE_HANDLE_REQUEST: c_ulong = (size_of::<HandleRequest>() as c_ulong) << SIZESHIFT;
const SIZE_EVENT_REQUEST: c_ulong = (size_of::<EventRequest>() as c_ulong) << SIZESHIFT;
const SIZE_HANDLE_DATA: c_ulong = (size_of::<HandleData>() as c_ulong) << SIZESHIFT;

const DIR_NONE: c_ulong = 0;
const DIR_WRITE: c_ulong = 1 << DIRSHIFT;
const DIR_READ: c_ulong = 2 << DIRSHIFT;
const DIR_READ_WRITE: c_ulong = DIR_READ | DIR_WRITE;

const REQ_GET_CHIP_INFO: c_ulong = DIR_READ | TYPE_GPIO | NR_GET_CHIP_INFO | SIZE_CHIP_INFO;
const REQ_GET_LINE_INFO: c_ulong = DIR_READ_WRITE | TYPE_GPIO | NR_GET_LINE_INFO | SIZE_LINE_INFO;
const REQ_GET_LINE_HANDLE: c_ulong =
    DIR_READ_WRITE | TYPE_GPIO | NR_GET_LINE_HANDLE | SIZE_HANDLE_REQUEST;
const REQ_GET_LINE_EVENT: c_ulong =
    DIR_READ_WRITE | TYPE_GPIO | NR_GET_LINE_EVENT | SIZE_EVENT_REQUEST;
const REQ_GET_LINE_VALUES: c_ulong =
    DIR_READ_WRITE | TYPE_GPIO | NR_GET_LINE_VALUES | SIZE_HANDLE_DATA;
const REQ_SET_LINE_VALUES: c_ulong =
    DIR_READ_WRITE | TYPE_GPIO | NR_SET_LINE_VALUES | SIZE_HANDLE_DATA;

// I'm not sure the GPIO header is always available through /dev/gpiochip0, so searching for
// the corresponding driver name in the label field seems like a more reliable option.
pub unsafe fn find_driver() -> Result<Option<File>> {
    let driver_name = b"pinctrl-bcm2835\0";

    let mut chip_info = ChipInfo::new();

    for idx in 0..=255 {
        let gpiochip = OpenOptions::new()
            .read(true)
            .write(true)
            .open(format!("/dev/gpiochip{}", idx))?;

        parse_retval(ioctl(
            gpiochip.as_raw_fd(),
            REQ_GET_CHIP_INFO,
            &mut chip_info,
        ))?;
        if chip_info.label[0..driver_name.len()] == driver_name[..] {
            return Ok(Some(gpiochip));
        }
    }

    Ok(None)
}

pub unsafe fn poll_interrupt(gpiochip: &mut File, pin: u8, trigger: Trigger) -> Result<Level> {
    let fd = gpiochip.as_raw_fd();

    let mut chip_info = ChipInfo::new();

    parse_retval(ioctl(fd, REQ_GET_CHIP_INFO, &mut chip_info))?;

    if u32::from(pin) > chip_info.lines || pin as usize >= HANDLES_MAX {
        return Err(Error::InvalidPin(pin));
    }

    let mut event_request = EventRequest::new(pin, HANDLE_FLAG_INPUT, trigger);

    parse_retval(ioctl(fd, REQ_GET_LINE_EVENT, &mut event_request))?;

    let poll = Epoll::new()?;
    poll.add(
        event_request.fd,
        event_request.fd as u64,
        EPOLLIN | EPOLLPRI,
    )?;

    let mut events = [epoll_event { events: 0, u64: 0 }; 1];
    loop {
        let num_events = poll.wait(&mut events, None)?;
        if num_events > 0 {
            for event in &events[0..num_events] {
                if event.u64 as c_int == event_request.fd {
                    let event_data = EventData::from_fd(event_request.fd)?;
                    match event_data.id {
                        EVENT_TYPE_RISING_EDGE => return Ok(Level::High),
                        _ => return Ok(Level::Low),
                    }
                }
            }
        }
    }
}

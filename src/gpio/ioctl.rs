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

use libc::{self, c_int, c_ulong, c_void, ENOENT};
use std::ffi::CString;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io;
use std::mem;
use std::os::unix::io::AsRawFd;
use std::time::Duration;

use crate::gpio::{Error, Level, Result, Trigger};

#[cfg(target_env = "gnu")]
type IoctlLong = libc::c_ulong;
#[cfg(target_env = "musl")]
type IoctlLong = c_int;

const PATH_GPIOCHIP: &str = "/dev/gpiochip";
const CONSUMER_LABEL: &str = "RPPAL";
const DRIVER_NAME: &[u8] = b"pinctrl-bcm2835\0";

const NRBITS: u8 = 8;
const TYPEBITS: u8 = 8;
const SIZEBITS: u8 = 14;
const DIRBITS: u8 = 2;

const NRSHIFT: u8 = 0;
const TYPESHIFT: u8 = (NRSHIFT + NRBITS);
const SIZESHIFT: u8 = (TYPESHIFT + TYPEBITS);
const DIRSHIFT: u8 = (SIZESHIFT + SIZEBITS);

const NR_GET_CHIP_INFO: IoctlLong = 0x01 << NRSHIFT;
const NR_GET_LINE_INFO: IoctlLong = 0x02 << NRSHIFT;
const NR_GET_LINE_HANDLE: IoctlLong = 0x03 << NRSHIFT;
const NR_GET_LINE_EVENT: IoctlLong = 0x04 << NRSHIFT;
const NR_GET_LINE_VALUES: IoctlLong = 0x08 << NRSHIFT;
const NR_SET_LINE_VALUES: IoctlLong = 0x09 << NRSHIFT;

const TYPE_GPIO: IoctlLong = (0xB4 as IoctlLong) << TYPESHIFT;

const SIZE_CHIP_INFO: IoctlLong = (mem::size_of::<ChipInfo>() as IoctlLong) << SIZESHIFT;
const SIZE_LINE_INFO: IoctlLong = (mem::size_of::<LineInfo>() as IoctlLong) << SIZESHIFT;
const SIZE_HANDLE_REQUEST: IoctlLong = (mem::size_of::<HandleRequest>() as IoctlLong) << SIZESHIFT;
const SIZE_EVENT_REQUEST: IoctlLong = (mem::size_of::<EventRequest>() as IoctlLong) << SIZESHIFT;
const SIZE_HANDLE_DATA: IoctlLong = (mem::size_of::<HandleData>() as IoctlLong) << SIZESHIFT;

const DIR_NONE: c_ulong = 0;
const DIR_WRITE: IoctlLong = 1 << DIRSHIFT;
const DIR_READ: IoctlLong = 2 << DIRSHIFT;
const DIR_READ_WRITE: IoctlLong = DIR_READ | DIR_WRITE;

const REQ_GET_CHIP_INFO: IoctlLong = DIR_READ | TYPE_GPIO | NR_GET_CHIP_INFO | SIZE_CHIP_INFO;
const REQ_GET_LINE_INFO: IoctlLong = DIR_READ_WRITE | TYPE_GPIO | NR_GET_LINE_INFO | SIZE_LINE_INFO;
const REQ_GET_LINE_HANDLE: IoctlLong =
    DIR_READ_WRITE | TYPE_GPIO | NR_GET_LINE_HANDLE | SIZE_HANDLE_REQUEST;
const REQ_GET_LINE_EVENT: IoctlLong =
    DIR_READ_WRITE | TYPE_GPIO | NR_GET_LINE_EVENT | SIZE_EVENT_REQUEST;
const REQ_GET_LINE_VALUES: IoctlLong =
    DIR_READ_WRITE | TYPE_GPIO | NR_GET_LINE_VALUES | SIZE_HANDLE_DATA;
const REQ_SET_LINE_VALUES: IoctlLong =
    DIR_READ_WRITE | TYPE_GPIO | NR_SET_LINE_VALUES | SIZE_HANDLE_DATA;

const NAME_BUFSIZE: usize = 32;
const LABEL_BUFSIZE: usize = 32;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct ChipInfo {
    pub name: [u8; NAME_BUFSIZE],
    pub label: [u8; LABEL_BUFSIZE],
    pub lines: u32,
}

impl ChipInfo {
    pub fn new(cdev_fd: c_int) -> Result<ChipInfo> {
        let mut chip_info = ChipInfo {
            name: [0u8; NAME_BUFSIZE],
            label: [0u8; LABEL_BUFSIZE],
            lines: 0,
        };

        parse_retval!(unsafe { libc::ioctl(cdev_fd, REQ_GET_CHIP_INFO, &mut chip_info) })?;

        Ok(chip_info)
    }
}

impl fmt::Debug for ChipInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ChipInfo")
            .field("name", &cbuf_to_cstring(&self.name))
            .field("label", &cbuf_to_cstring(&self.label))
            .field("lines", &self.lines)
            .finish()
    }
}

const LINE_FLAG_KERNEL: u32 = 0x01;
const LINE_FLAG_IS_OUT: u32 = 0x02;
const LINE_FLAG_ACTIVE_LOW: u32 = 0x04;
const LINE_FLAG_OPEN_DRAIN: u32 = 0x08;
const LINE_FLAG_OPEN_SOURCE: u32 = 0x10;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct LineInfo {
    pub line_offset: u32,
    pub flags: u32,
    pub name: [u8; NAME_BUFSIZE],
    pub consumer: [u8; LABEL_BUFSIZE],
}

impl LineInfo {
    pub fn new() -> LineInfo {
        LineInfo {
            line_offset: 0,
            flags: 0,
            name: [0u8; NAME_BUFSIZE],
            consumer: [0u8; LABEL_BUFSIZE],
        }
    }
}

impl fmt::Debug for LineInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LineInfo")
            .field("line_offset", &self.line_offset)
            .field("flags", &self.flags)
            .field("name", &cbuf_to_cstring(&self.name))
            .field("consumer", &cbuf_to_cstring(&self.consumer))
            .finish()
    }
}

const HANDLES_MAX: usize = 64;
const HANDLE_FLAG_INPUT: u32 = 0x01;
const HANDLE_FLAG_OUTPUT: u32 = 0x02;
const HANDLE_FLAG_ACTIVE_LOW: u32 = 0x04;
const HANDLE_FLAG_OPEN_DRAIN: u32 = 0x08;
const HANDLE_FLAG_OPEN_SOURCE: u32 = 0x10;

#[repr(C)]
pub struct HandleRequest {
    pub line_offsets: [u32; HANDLES_MAX],
    pub flags: u32,
    pub default_values: [u8; HANDLES_MAX],
    pub consumer_label: [u8; LABEL_BUFSIZE],
    pub lines: u32,
    pub fd: c_int,
}

impl HandleRequest {
    pub fn new(cdev_fd: c_int, pins: &[u8]) -> Result<HandleRequest> {
        let mut handle_request = HandleRequest {
            line_offsets: [0u32; HANDLES_MAX],
            flags: 0,
            default_values: [0u8; HANDLES_MAX],
            consumer_label: [0u8; LABEL_BUFSIZE],
            lines: 0,
            fd: 0,
        };

        let pins: &[u8] = if pins.len() > HANDLES_MAX {
            handle_request.lines = HANDLES_MAX as u32;
            &pins[0..HANDLES_MAX]
        } else {
            handle_request.lines = pins.len() as u32;
            pins
        };

        for (idx, pin) in pins.iter().enumerate() {
            handle_request.line_offsets[idx] = u32::from(*pin);
        }

        // Set consumer label, so other processes know we're using these pins
        handle_request.consumer_label[0..CONSUMER_LABEL.len()]
            .copy_from_slice(CONSUMER_LABEL.as_bytes());

        parse_retval!(unsafe { libc::ioctl(cdev_fd, REQ_GET_LINE_HANDLE, &mut handle_request) })?;

        // If the handle fd is zero or negative, an error occurred
        if handle_request.fd <= 0 {
            Err(Error::Io(std::io::Error::last_os_error()))
        } else {
            Ok(handle_request)
        }
    }

    pub fn levels(&self) -> Result<HandleData> {
        let mut handle_data = HandleData::new();

        parse_retval!(unsafe { libc::ioctl(self.fd, REQ_GET_LINE_VALUES, &mut handle_data) })?;

        Ok(handle_data)
    }

    pub fn set_levels(&mut self, levels: &[Level]) -> Result<()> {
        let mut handle_data = HandleData::new();
        let levels: &[Level] = if levels.len() > HANDLES_MAX {
            &levels[0..HANDLES_MAX]
        } else {
            levels
        };

        for (idx, level) in levels.iter().enumerate() {
            handle_data.values[idx] = *level as u8;
        }

        parse_retval!(unsafe { libc::ioctl(self.fd, REQ_SET_LINE_VALUES, &mut handle_data) })?;

        Ok(())
    }

    pub fn close(&mut self) {
        if self.fd > 0 {
            unsafe {
                libc::close(self.fd);
            }

            self.fd = 0;
        }
    }
}

impl Drop for HandleRequest {
    fn drop(&mut self) {
        self.close();
    }
}

impl fmt::Debug for HandleRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HandleRequest")
            .field(
                "line_offsets",
                &format_args!("{:?}", &self.line_offsets[..self.lines as usize]),
            )
            .field("flags", &self.flags)
            .field(
                "default_values",
                &format_args!("{:?}", &self.default_values[..self.lines as usize]),
            )
            .field("consumer_label", &cbuf_to_cstring(&self.consumer_label))
            .field("lines", &self.lines)
            .field("fd", &self.fd)
            .finish()
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct HandleData {
    pub values: [u8; HANDLES_MAX],
}

impl HandleData {
    pub fn new() -> HandleData {
        HandleData {
            values: [0u8; HANDLES_MAX],
        }
    }
}

impl fmt::Debug for HandleData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HandleRequest")
            .field("values", &&self.values[..])
            .finish()
    }
}

const EVENT_FLAG_RISING_EDGE: u32 = 0x01;
const EVENT_FLAG_FALLING_EDGE: u32 = 0x02;
const EVENT_FLAG_BOTH_EDGES: u32 = EVENT_FLAG_RISING_EDGE | EVENT_FLAG_FALLING_EDGE;

#[repr(C)]
pub struct EventRequest {
    pub line_offset: u32,
    pub handle_flags: u32,
    pub event_flags: u32,
    pub consumer_label: [u8; LABEL_BUFSIZE],
    pub fd: c_int,
}

impl EventRequest {
    pub fn new(cdev_fd: c_int, pin: u8, trigger: Trigger) -> Result<EventRequest> {
        let mut event_request = EventRequest {
            line_offset: u32::from(pin),
            handle_flags: HANDLE_FLAG_INPUT,
            event_flags: trigger as u32,
            consumer_label: [0u8; LABEL_BUFSIZE],
            fd: 0,
        };

        // Set consumer label, so other processes know we're monitoring this event
        event_request.consumer_label[0..CONSUMER_LABEL.len()]
            .copy_from_slice(CONSUMER_LABEL.as_bytes());

        parse_retval!(unsafe { libc::ioctl(cdev_fd, REQ_GET_LINE_EVENT, &mut event_request) })?;

        // If the event fd is zero or negative, an error occurred
        if event_request.fd <= 0 {
            Err(Error::Io(std::io::Error::last_os_error()))
        } else {
            Ok(event_request)
        }
    }

    pub fn close(&mut self) {
        if self.fd > 0 {
            unsafe {
                libc::close(self.fd);
            }

            self.fd = 0;
        }
    }
}

impl Drop for EventRequest {
    fn drop(&mut self) {
        self.close();
    }
}

impl fmt::Debug for EventRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventRequest")
            .field("line_offset", &self.line_offset)
            .field("handle_flags", &self.handle_flags)
            .field("event_flags", &self.event_flags)
            .field("consumer_label", &cbuf_to_cstring(&self.consumer_label))
            .field("fd", &self.fd)
            .finish()
    }
}

const EVENT_TYPE_RISING_EDGE: u32 = 0x01;
const EVENT_TYPE_FALLING_EDGE: u32 = 0x02;

#[derive(Debug, Copy, Clone, Default)]
#[repr(C)]
struct EventData {
    timestamp: u64,
    id: u32,
}

impl EventData {
    fn new(event_fd: c_int) -> Result<EventData> {
        let mut event_data = EventData {
            timestamp: 0,
            id: 0,
        };

        let bytes_read = parse_retval!(unsafe {
            libc::read(
                event_fd,
                &mut event_data as *mut EventData as *mut c_void,
                mem::size_of::<EventData>(),
            )
        })?;

        if bytes_read < mem::size_of::<EventData>() as isize {
            Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "failed to fill whole buffer",
            )
            .into())
        } else {
            Ok(event_data)
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Event {
    pub trigger: Trigger,
    pub timestamp: Duration,
}

impl Event {
    fn from_event_data(event_data: EventData) -> Event {
        Event {
            trigger: match event_data.id {
                EVENT_TYPE_RISING_EDGE => Trigger::RisingEdge,
                EVENT_TYPE_FALLING_EDGE => Trigger::FallingEdge,
                _ => unreachable!(),
            },
            timestamp: Duration::from_nanos(event_data.timestamp),
        }
    }
}

// Read interrupt event
pub fn get_event(event_fd: c_int) -> Result<Event> {
    let event_data = EventData::new(event_fd)?;
    Ok(Event::from_event_data(event_data))
}

// Find the correct gpiochip device based on its label
pub fn find_gpiochip() -> Result<File> {
    for id in 0..=255 {
        let gpiochip = match OpenOptions::new()
            .read(true)
            .write(true)
            .open(format!("{}{}", PATH_GPIOCHIP, id))
        {
            Ok(file) => file,
            Err(ref e) if e.kind() == io::ErrorKind::PermissionDenied => {
                return Err(Error::PermissionDenied(format!("{}{}", PATH_GPIOCHIP, id)));
            }
            Err(e) => return Err(Error::from(e)),
        };

        let chip_info = ChipInfo::new(gpiochip.as_raw_fd())?;
        if chip_info.label[0..DRIVER_NAME.len()] == DRIVER_NAME[..] {
            return Ok(gpiochip);
        }
    }

    // File Not Found I/O error
    Err(Error::Io(io::Error::from_raw_os_error(ENOENT)))
}

// Create a CString from a C-style NUL-terminated char array. This workaround
// is needed for fixed-length buffers that fill the remaining bytes with NULs,
// because CString::new() interprets those as a NUL in the middle of the byte
// slice and returns a NulError.
fn cbuf_to_cstring(buf: &[u8]) -> CString {
    CString::new({
        let pos = buf
            .iter()
            .position(|&c| c == b'\0')
            .unwrap_or_else(|| buf.len());
        &buf[..pos]
    })
    .unwrap_or_default()
}

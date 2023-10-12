#![allow(clippy::unnecessary_cast)]
#![allow(dead_code)]

use crate::gpio::{Error, Level, Result, Trigger};
use libc::{self, c_int, c_void, ENOENT};
use std::ffi::CString;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io;
use std::mem;
use std::os::unix::io::AsRawFd;
use std::time::Duration;

#[cfg(target_env = "gnu")]
type IoctlLong = libc::c_ulong;
#[cfg(target_env = "musl")]
type IoctlLong = c_int;

const PATH_GPIOCHIP: &str = "/dev/gpiochip";
const CONSUMER_LABEL: &str = "RPPAL";
const DRIVER_NAME: &[u8] = b"pinctrl-bcm2835\0";
const DRIVER_NAME_BCM2711: &[u8] = b"pinctrl-bcm2711\0";
const DRIVER_NAME_BCM2712: &[u8] = b"pinctrl-rp1\0";

// The first 27 offsets correspond to the 40-pin header
pub const MAX_OFFSET: u32 = 27;

const BITS_NR: u8 = 8;
const BITS_TYPE: u8 = 8;
const BITS_SIZE: u8 = 14;
const BITS_DIR: u8 = 2;

const SHIFT_NR: u8 = 0;
const SHIFT_TYPE: u8 = SHIFT_NR + BITS_NR;
const SHIFT_SIZE: u8 = SHIFT_TYPE + BITS_TYPE;
const SHIFT_DIR: u8 = SHIFT_SIZE + BITS_SIZE;

const DIR_NONE: IoctlLong = 0;
const DIR_WRITE: IoctlLong = 1 << SHIFT_DIR;
const DIR_READ: IoctlLong = 2 << SHIFT_DIR;
const DIR_READ_WRITE: IoctlLong = DIR_READ | DIR_WRITE;

const TYPE_GPIO: IoctlLong = (0xB4 as IoctlLong) << SHIFT_TYPE;

const NR_GET_CHIP_INFO: IoctlLong = 0x01 << SHIFT_NR;
const NR_GET_LINE_INFO: IoctlLong = 0x05 << SHIFT_NR;
const NR_GET_LINE_INFO_WATCH: IoctlLong = 0x06 << SHIFT_NR;
const NR_GET_LINE_INFO_UNWATCH: IoctlLong = 0x0C << SHIFT_NR;
const NR_GET_LINE: IoctlLong = 0x07 << SHIFT_NR;
const NR_LINE_SET_CONFIG: IoctlLong = 0x0D << SHIFT_NR;
const NR_LINE_GET_VALUES: IoctlLong = 0x0E << SHIFT_NR;
const NR_LINE_SET_VALUES: IoctlLong = 0x0F << SHIFT_NR;

const SIZE_CHIP_INFO: IoctlLong = (mem::size_of::<ChipInfo>() as IoctlLong) << SHIFT_SIZE;
const SIZE_LINE_INFO: IoctlLong = (mem::size_of::<LineInfo>() as IoctlLong) << SHIFT_SIZE;
const SIZE_U32: IoctlLong = (mem::size_of::<u32>() as IoctlLong) << SHIFT_SIZE;
const SIZE_LINE_REQUEST: IoctlLong = (mem::size_of::<LineRequest>() as IoctlLong) << SHIFT_SIZE;
const SIZE_LINE_CONFIG: IoctlLong = (mem::size_of::<LineConfig>() as IoctlLong) << SHIFT_SIZE;
const SIZE_LINE_VALUES: IoctlLong = (mem::size_of::<LineValues>() as IoctlLong) << SHIFT_SIZE;

const GPIO_GET_CHIPINFO_IOCTL: IoctlLong = DIR_READ | TYPE_GPIO | NR_GET_CHIP_INFO | SIZE_CHIP_INFO;
const GPIO_V2_GET_LINEINFO_IOCTL: IoctlLong =
    DIR_READ_WRITE | TYPE_GPIO | NR_GET_LINE_INFO | SIZE_LINE_INFO;
const GPIO_V2_GET_LINEINFO_WATCH_IOCTL: IoctlLong =
    DIR_READ_WRITE | TYPE_GPIO | NR_GET_LINE_INFO_WATCH | SIZE_LINE_INFO;
const GPIO_GET_LINEINFO_UNWATCH_IOCTL: IoctlLong =
    DIR_READ_WRITE | TYPE_GPIO | NR_GET_LINE_INFO_UNWATCH | SIZE_U32;
const GPIO_V2_GET_LINE_IOCTL: IoctlLong =
    DIR_READ_WRITE | TYPE_GPIO | NR_GET_LINE | SIZE_LINE_REQUEST;
const GPIO_V2_LINE_SET_CONFIG_IOCTL: IoctlLong =
    DIR_READ_WRITE | TYPE_GPIO | NR_LINE_SET_CONFIG | SIZE_LINE_CONFIG;
const GPIO_V2_LINE_GET_VALUES_IOCTL: IoctlLong =
    DIR_READ_WRITE | TYPE_GPIO | NR_LINE_GET_VALUES | SIZE_LINE_VALUES;
const GPIO_V2_LINE_SET_VALUES_IOCTL: IoctlLong =
    DIR_READ_WRITE | TYPE_GPIO | NR_LINE_SET_VALUES | SIZE_LINE_VALUES;

// Maximum name and label length.
const NAME_BUFSIZE: usize = 32;
const LABEL_BUFSIZE: usize = 32;

// Maximum number of requested lines.
const LINES_MAX: usize = 64;
// Maximum number of configuration attributes.
const LINE_NUM_ATTRS_MAX: usize = 10;

const LINE_FLAG_USED: u64 = 0x01;
const LINE_FLAG_ACTIVE_LOW: u64 = 0x02;
const LINE_FLAG_INPUT: u64 = 0x04;
const LINE_FLAG_OUTPUT: u64 = 0x08;
const LINE_FLAG_EDGE_RISING: u64 = 0x10;
const LINE_FLAG_EDGE_FALLING: u64 = 0x20;
const LINE_FLAG_OPEN_DRAIN: u64 = 0x40;
const LINE_FLAG_OPEN_SOURCE: u64 = 0x80;
const LINE_FLAG_BIAS_PULL_UP: u64 = 0x1000;
const LINE_FLAG_BIAS_PULL_DOWN: u64 = 0x2000;
const LINE_FLAG_BIAS_DISABLED: u64 = 0x4000;
const LINE_FLAG_EVENT_CLOCK_REALTIME: u64 = 0x8000;
const LINE_FLAG_EVENT_CLOCK_HTE: u64 = 0x100000;

const LINE_ATTR_ID_FLAGS: u32 = 1;
const LINE_ATTR_ID_OUTPUT_VALUES: u32 = 2;
const LINE_ATTR_ID_DEBOUNCE: u32 = 3;

const LINE_CHANGED_REQUESTED: u32 = 1;
const LINE_CHANGED_RELEASED: u32 = 2;
const LINE_CHANGED_CONFIG: u32 = 3;

const LINE_EVENT_RISING_EDGE: u32 = 1;
const LINE_EVENT_FALLING_EDGE: u32 = 2;

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

        parse_retval!(unsafe { libc::ioctl(cdev_fd, GPIO_GET_CHIPINFO_IOCTL, &mut chip_info) })?;

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

pub struct LineFlags {
    flags: u64,
}

impl fmt::Debug for LineFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LineFlags")
            .field("flags", &self.flags)
            .finish()
    }
}

impl fmt::Display for LineFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut string_list = Vec::new();

        if self.used() {
            string_list.push("used");
        }

        if self.active_low() {
            string_list.push("active_low");
        }

        if self.input() {
            string_list.push("input");
        }

        if self.output() {
            string_list.push("output");
        }

        if self.edge_rising() {
            string_list.push("edge_rising");
        }

        if self.edge_falling() {
            string_list.push("edge_falling");
        }

        if self.open_drain() {
            string_list.push("open_drain");
        }

        if self.open_source() {
            string_list.push("open_source");
        }

        if self.bias_pull_up() {
            string_list.push("bias_pull_up");
        }

        if self.bias_pull_down() {
            string_list.push("bias_pull_down");
        }

        if self.bias_disabled() {
            string_list.push("bias_disabled");
        }

        if self.event_clock_realtime() {
            string_list.push("event_clock_realtime");
        }

        if self.event_clock_hte() {
            string_list.push("event_clock_hte");
        }

        write!(f, "{}", string_list.join(" "))
    }
}

impl LineFlags {
    pub fn new(flags: u64) -> LineFlags {
        LineFlags { flags }
    }

    pub fn used(&self) -> bool {
        (self.flags & LINE_FLAG_USED) > 0
    }

    pub fn active_low(&self) -> bool {
        (self.flags & LINE_FLAG_ACTIVE_LOW) > 0
    }

    pub fn input(&self) -> bool {
        (self.flags & LINE_FLAG_INPUT) > 0
    }

    pub fn output(&self) -> bool {
        (self.flags & LINE_FLAG_OUTPUT) > 0
    }

    pub fn edge_rising(&self) -> bool {
        (self.flags & LINE_FLAG_EDGE_RISING) > 0
    }

    pub fn edge_falling(&self) -> bool {
        (self.flags & LINE_FLAG_EDGE_FALLING) > 0
    }

    pub fn open_drain(&self) -> bool {
        (self.flags & LINE_FLAG_OPEN_DRAIN) > 0
    }

    pub fn open_source(&self) -> bool {
        (self.flags & LINE_FLAG_OPEN_SOURCE) > 0
    }

    pub fn bias_pull_up(&self) -> bool {
        (self.flags & LINE_FLAG_BIAS_PULL_UP) > 0
    }

    pub fn bias_pull_down(&self) -> bool {
        (self.flags & LINE_FLAG_BIAS_PULL_DOWN) > 0
    }

    pub fn bias_disabled(&self) -> bool {
        (self.flags & LINE_FLAG_EDGE_FALLING) > 0
    }

    pub fn event_clock_realtime(&self) -> bool {
        (self.flags & LINE_FLAG_EVENT_CLOCK_REALTIME) > 0
    }

    pub fn event_clock_hte(&self) -> bool {
        (self.flags & LINE_FLAG_EVENT_CLOCK_HTE) > 0
    }
}

#[derive(Copy, Clone, Default)]
#[repr(C)]
pub struct LineAttribute {
    pub id: u32,
    pub padding: u32,
    pub values: u64,
}

impl LineAttribute {
    pub fn new() -> LineAttribute {
        LineAttribute {
            id: 0,
            padding: 0,
            values: 0,
        }
    }
}

impl fmt::Debug for LineAttribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LineAttribute")
            .field("id", &self.id)
            .field("padding", &self.padding)
            .field("values", &self.values)
            .finish()
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct LineInfo {
    pub name: [u8; NAME_BUFSIZE],
    pub consumer: [u8; LABEL_BUFSIZE],
    pub offset: u32,
    pub num_attrs: u32,
    pub flags: u64,
    pub attrs: [LineAttribute; LINE_NUM_ATTRS_MAX],
    pub padding: [u32; 4],
}

impl LineInfo {
    pub fn new(cdev_fd: c_int, offset: u32) -> Result<LineInfo> {
        let mut line_info = LineInfo {
            name: [0u8; NAME_BUFSIZE],
            consumer: [0u8; LABEL_BUFSIZE],
            offset,
            num_attrs: 0,
            flags: 0,
            attrs: [LineAttribute::new(); 10],
            padding: [0u32; 4],
        };

        parse_retval!(unsafe { libc::ioctl(cdev_fd, GPIO_V2_GET_LINEINFO_IOCTL, &mut line_info) })?;

        Ok(line_info)
    }

    pub fn flags(&self) -> LineFlags {
        LineFlags::new(self.flags)
    }
}

impl fmt::Debug for LineInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LineInfo")
            .field("name", &cbuf_to_cstring(&self.name))
            .field("consumer", &cbuf_to_cstring(&self.consumer))
            .field("offset", &self.offset)
            .field("num_attrs", &self.num_attrs)
            .field("flags", &self.flags)
            .field("attrs", &self.attrs)
            .field("padding", &self.padding)
            .finish()
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct LineInfoChanged {
    pub info: LineInfo,
    pub timestamp_ns: u64,
    pub event_type: u32,
    pub padding: [u32; 5],
}

#[derive(Copy, Clone, Default)]
#[repr(C)]
pub struct LineConfigAttribute {
    pub attr: LineAttribute,
    pub mask: u64,
}

impl fmt::Debug for LineConfigAttribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LineConfigAttribute")
            .field("attr", &self.attr)
            .field("mask", &self.mask)
            .finish()
    }
}

#[derive(Copy, Clone, Default)]
#[repr(C)]
pub struct LineConfig {
    pub flags: u64,
    pub num_attrs: u32,
    pub padding: [u32; 5],
    pub attrs: [LineConfigAttribute; LINE_NUM_ATTRS_MAX],
}

impl fmt::Debug for LineConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LineConfig")
            .field("flags", &self.flags)
            .field("num_attrs", &self.num_attrs)
            .field("padding", &self.padding)
            .field("attrs", &self.attrs)
            .finish()
    }
}

#[derive(Clone)]
#[repr(C)]
pub struct LineRequest {
    pub offsets: [u32; LINES_MAX],
    pub consumer: [u8; LABEL_BUFSIZE],
    pub config: LineConfig,
    pub num_lines: u32,
    pub event_buffer_size: u32,
    pub padding: [u32; 5],
    pub fd: c_int,
}

impl Default for LineRequest {
    fn default() -> Self {
        Self {
            offsets: [0u32; LINES_MAX],
            consumer: [0u8; LABEL_BUFSIZE],
            config: Default::default(),
            num_lines: Default::default(),
            event_buffer_size: 0,
            padding: [0u32; 5],
            fd: 0,
        }
    }
}

impl fmt::Debug for LineRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LineRequest")
            .field("offsets", &self.offsets)
            .field("consumer", &cbuf_to_cstring(&self.consumer))
            .field("config", &self.config)
            .field("num_lines", &self.num_lines)
            .field("event_buffer_size", &self.event_buffer_size)
            .field("padding", &self.padding)
            .field("fd", &self.fd)
            .finish()
    }
}

impl LineRequest {
    pub fn new(cdev_fd: c_int, offset: u32) -> Result<LineRequest> {
        let mut line_request = LineRequest::default();
        line_request.offsets[0] = offset;
        line_request.num_lines = 1;

        // Set consumer label, so other processes know we're monitoring this event
        line_request.consumer[0..CONSUMER_LABEL.len()].copy_from_slice(CONSUMER_LABEL.as_bytes());

        parse_retval!(unsafe { libc::ioctl(cdev_fd, GPIO_V2_GET_LINE_IOCTL, &mut line_request) })?;

        // If the fd is zero or negative, an error occurred
        if line_request.fd <= 0 {
            Err(Error::Io(std::io::Error::last_os_error()))
        } else {
            Ok(line_request)
        }
    }

    pub fn levels(&self) -> Result<LineValues> {
        let mut line_values = LineValues::new(0, 0x01);

        parse_retval!(unsafe {
            libc::ioctl(self.fd, GPIO_V2_LINE_GET_VALUES_IOCTL, &mut line_values)
        })?;

        Ok(line_values)
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

impl Drop for LineRequest {
    fn drop(&mut self) {
        self.close();
    }
}

#[derive(Copy, Clone, Default)]
#[repr(C)]
pub struct LineValues {
    pub bits: u64,
    pub mask: u64,
}

impl fmt::Debug for LineValues {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LineValues")
            .field("bits", &self.bits)
            .field("mask", &self.mask)
            .finish()
    }
}

impl LineValues {
    pub fn new(bits: u64, mask: u64) -> LineValues {
        LineValues { bits, mask }
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct LineEvent {
    pub timestamp_ns: u64,
    pub id: u32,
    pub offset: u32,
    pub seqno: u32,
    pub line_seqno: u32,
    pub padding: [u32; 6],
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
        if chip_info.label[0..DRIVER_NAME.len()] == DRIVER_NAME[..]
            || chip_info.label[0..DRIVER_NAME_BCM2711.len()] == DRIVER_NAME_BCM2711[..]
            || chip_info.label[0..DRIVER_NAME_BCM2712.len()] == DRIVER_NAME_BCM2712[..]
        {
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
pub fn cbuf_to_cstring(buf: &[u8]) -> CString {
    CString::new({
        let pos = buf.iter().position(|&c| c == b'\0').unwrap_or(buf.len());
        &buf[..pos]
    })
    .unwrap_or_default()
}

pub fn cbuf_to_string(buf: &[u8]) -> String {
    cbuf_to_cstring(buf).into_string().unwrap_or_default()
}

// Deprecated v1 API requests
const NR_GET_LINE_EVENT: IoctlLong = 0x04 << SHIFT_NR;

const SIZE_EVENT_REQUEST: IoctlLong = (mem::size_of::<EventRequest>() as IoctlLong) << SHIFT_SIZE;

const GPIO_GET_LINEEVENT_IOCTL: IoctlLong =
    DIR_READ_WRITE | TYPE_GPIO | NR_GET_LINE_EVENT | SIZE_EVENT_REQUEST;

const HANDLE_FLAG_INPUT: u32 = 0x01;

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

        parse_retval!(unsafe {
            libc::ioctl(cdev_fd, GPIO_GET_LINEEVENT_IOCTL, &mut event_request)
        })?;

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
    trigger: Trigger,
    timestamp: Duration,
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

    pub fn trigger(&self) -> Trigger {
        self.trigger
    }

    pub fn level(&self) -> Level {
        match self.trigger {
            Trigger::RisingEdge => Level::High,
            Trigger::FallingEdge => Level::Low,
            _ => {
                // SAFETY: `Event` can only be constructed with either `RisingEdge` or `FallingEdge`.
                unsafe { std::hint::unreachable_unchecked() }
            }
        }
    }
}

// Read interrupt event
pub fn get_event(event_fd: c_int) -> Result<Event> {
    let event_data = EventData::new(event_fd)?;
    Ok(Event::from_event_data(event_data))
}

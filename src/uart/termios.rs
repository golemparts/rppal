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

use std::io;
use std::time::Duration;

use libc::{c_int, termios};
use libc::{B0, B110, B134, B150, B200, B300, B50, B75};
use libc::{B1000000, B1152000, B460800, B500000, B576000, B921600};
use libc::{B115200, B19200, B230400, B38400, B57600};
use libc::{B1200, B1800, B2400, B4800, B600, B9600};
use libc::{B1500000, B2000000, B2500000, B3000000, B3500000, B4000000};
use libc::{CLOCAL, CMSPAR, CREAD, CRTSCTS, TCSANOW};
use libc::{CS5, CS6, CS7, CS8, CSIZE, CSTOPB, PARENB, PARODD};
use libc::{IGNPAR, INPCK, PARMRK};
use libc::{IXANY, IXOFF, IXON, TCIFLUSH, TCIOFLUSH, TCOFLUSH, VMIN, VSTART, VSTOP, VTIME};
use libc::{TCIOFF, TCION, TIOCMGET, TIOCM_CTS, TIOCM_DSR, TIOCM_DTR, TIOCM_RTS};
use libc::{TIOCINQ, TIOCMBIC, TIOCMBIS, TIOCM_CAR, TIOCM_RNG, TIOCOUTQ};

use crate::uart::{Error, Parity, ParityCheck, Queue, Result};

const XON: u8 = 17;
const XOFF: u8 = 19;

#[cfg(target_env = "gnu")]
pub fn attributes(fd: c_int) -> Result<termios> {
    let mut attr = termios {
        c_iflag: 0,
        c_oflag: 0,
        c_cflag: 0,
        c_lflag: 0,
        c_line: 0,
        c_cc: [0u8; 32],
        c_ispeed: 0,
        c_ospeed: 0,
    };

    parse_retval!(unsafe { libc::tcgetattr(fd, &mut attr) })?;

    Ok(attr)
}

#[cfg(target_env = "musl")]
pub fn attributes(fd: c_int) -> Result<termios> {
    let mut attr = termios {
        c_iflag: 0,
        c_oflag: 0,
        c_cflag: 0,
        c_lflag: 0,
        c_line: 0,
        c_cc: [0u8; 32],
        __c_ispeed: 0,
        __c_ospeed: 0,
    };

    parse_retval!(unsafe { libc::tcgetattr(fd, &mut attr) })?;

    Ok(attr)
}

pub fn set_attributes(fd: c_int, attr: &termios) -> Result<()> {
    parse_retval!(unsafe { libc::tcsetattr(fd, TCSANOW, attr) }).map_err(|e| {
        if e.kind() == io::ErrorKind::InvalidInput {
            Error::InvalidValue
        } else {
            Error::Io(e)
        }
    })?;

    Ok(())
}

pub fn line_speed(fd: c_int) -> Result<u32> {
    // We can't use termios2 here, because it's not supported by musl
    Ok(match unsafe { libc::cfgetospeed(&attributes(fd)?) } {
        B0 => 0,
        B50 => 50,
        B75 => 75,
        B110 => 110,
        B134 => 134,
        B150 => 150,
        B200 => 200,
        B300 => 300,
        B600 => 600,
        B1200 => 1_200,
        B1800 => 1_800,
        B2400 => 2_400,
        B4800 => 4_800,
        B9600 => 9_600,
        B19200 => 19_200,
        B38400 => 38_400,
        B57600 => 57_600,
        B115200 => 115_200,
        B230400 => 230_400,
        B460800 => 460_800,
        B500000 => 500_000,
        B576000 => 576_000,
        B921600 => 921_600,
        B1000000 => 1_000_000,
        B1152000 => 1_152_000,
        B1500000 => 1_500_000,
        B2000000 => 2_000_000,
        B2500000 => 2_500_000,
        B3000000 => 3_000_000,
        B3500000 => 3_500_000,
        B4000000 => 4_000_000,
        _ => return Err(Error::InvalidValue),
    })
}

pub fn set_line_speed(fd: c_int, line_speed: u32) -> Result<()> {
    let baud = match line_speed {
        0 => B0,
        50 => B50,
        75 => B75,
        110 => B110,
        134 => B134,
        150 => B150,
        200 => B200,
        300 => B300,
        600 => B600,
        1_200 => B1200,
        1_800 => B1800,
        2_400 => B2400,
        4_800 => B4800,
        9_600 => B9600,
        19_200 => B19200,
        38_400 => B38400,
        57_600 => B57600,
        115_200 => B115200,
        230_400 => B230400,
        460_800 => B460800,
        500_000 => B500000,
        576_000 => B576000,
        921_600 => B921600,
        1_000_000 => B1000000,
        1_152_000 => B1152000,
        1_500_000 => B1500000,
        2_000_000 => B2000000,
        2_500_000 => B2500000,
        3_000_000 => B3000000,
        3_500_000 => B3500000,
        4_000_000 => B4000000,
        _ => return Err(Error::InvalidValue),
    };

    // We can't use termios2 here, because it's not supported by musl
    let mut attr = attributes(fd)?;

    parse_retval!(unsafe { libc::cfsetispeed(&mut attr, baud) })?;
    parse_retval!(unsafe { libc::cfsetospeed(&mut attr, baud) })?;

    set_attributes(fd, &attr)
}

pub fn parity(fd: c_int) -> Result<Parity> {
    let attr = attributes(fd)?;

    if (attr.c_cflag & PARENB) == 0 {
        return Ok(Parity::None);
    } else if (attr.c_cflag & PARENB) > 0 && (attr.c_cflag & PARODD) == 0 {
        return Ok(Parity::Even);
    } else if (attr.c_cflag & PARENB) > 0 && (attr.c_cflag & PARODD) > 0 {
        return Ok(Parity::Odd);
    } else if (attr.c_cflag & PARENB) > 0
        && (attr.c_cflag & CMSPAR) > 0
        && (attr.c_cflag & PARODD) > 0
    {
        return Ok(Parity::Mark);
    } else if (attr.c_cflag & PARENB) > 0
        && (attr.c_cflag & CMSPAR) > 0
        && (attr.c_cflag & PARODD) == 0
    {
        return Ok(Parity::Space);
    }

    Ok(Parity::None)
}

pub fn set_parity(fd: c_int, parity: Parity) -> Result<()> {
    let mut attr = attributes(fd)?;

    match parity {
        Parity::None => {
            attr.c_cflag &= !PARENB;
            attr.c_cflag &= !PARODD;
        }
        Parity::Even => {
            attr.c_cflag |= PARENB;
            attr.c_cflag &= !PARODD;
        }
        Parity::Odd => {
            attr.c_cflag |= PARENB | PARODD;
        }
        Parity::Mark => {
            attr.c_cflag |= PARENB | PARODD | CMSPAR;
        }
        Parity::Space => {
            attr.c_cflag |= PARENB | CMSPAR;
            attr.c_cflag &= !PARODD;
        }
    }

    set_attributes(fd, &attr)
}

pub fn parity_check(fd: c_int) -> Result<ParityCheck> {
    let attr = attributes(fd)?;

    if (attr.c_iflag & INPCK) == 0 {
        return Ok(ParityCheck::None);
    } else if (attr.c_iflag & INPCK) > 0
        && (attr.c_iflag & IGNPAR) > 0
        && (attr.c_iflag & PARMRK) == 0
    {
        return Ok(ParityCheck::Strip);
    } else if (attr.c_iflag & INPCK) > 0
        && (attr.c_iflag & IGNPAR) == 0
        && (attr.c_iflag & PARMRK) == 0
    {
        return Ok(ParityCheck::Replace);
    } else if (attr.c_iflag & INPCK) > 0
        && (attr.c_iflag & IGNPAR) == 0
        && (attr.c_iflag & PARMRK) > 0
    {
        return Ok(ParityCheck::Mark);
    }

    Ok(ParityCheck::None)
}

pub fn set_parity_check(fd: c_int, parity_check: ParityCheck) -> Result<()> {
    let mut attr = attributes(fd)?;

    match parity_check {
        ParityCheck::None => attr.c_iflag &= !(INPCK | IGNPAR | PARMRK),
        ParityCheck::Strip => {
            attr.c_iflag |= INPCK | IGNPAR;
            attr.c_iflag &= !PARMRK;
        }
        ParityCheck::Replace => {
            attr.c_iflag |= INPCK;
            attr.c_iflag &= !(IGNPAR | PARMRK);
        }
        ParityCheck::Mark => {
            attr.c_iflag |= INPCK | PARMRK;
            attr.c_iflag &= !IGNPAR;
        }
    }

    set_attributes(fd, &attr)
}

pub fn data_bits(fd: c_int) -> Result<u8> {
    let attr = attributes(fd)?;

    Ok(match attr.c_cflag & CSIZE {
        CS5 => 5,
        CS6 => 6,
        CS7 => 7,
        CS8 => 8,
        _ => return Err(Error::InvalidValue),
    })
}

pub fn set_data_bits(fd: c_int, data_bits: u8) -> Result<()> {
    let mut attr = attributes(fd)?;

    attr.c_cflag &= !CSIZE;
    match data_bits {
        5 => attr.c_cflag |= CS5,
        6 => attr.c_cflag |= CS6,
        7 => attr.c_cflag |= CS7,
        8 => attr.c_cflag |= CS8,
        _ => return Err(Error::InvalidValue),
    }

    set_attributes(fd, &attr)
}

pub fn stop_bits(fd: c_int) -> Result<u8> {
    let attr = attributes(fd)?;

    if (attr.c_cflag & CSTOPB) > 0 {
        Ok(2)
    } else {
        Ok(1)
    }
}

pub fn set_stop_bits(fd: c_int, stop_bits: u8) -> Result<()> {
    let mut attr = attributes(fd)?;

    match stop_bits {
        1 => attr.c_cflag &= !CSTOPB,
        2 => attr.c_cflag |= CSTOPB,
        _ => return Err(Error::InvalidValue),
    }

    set_attributes(fd, &attr)
}

// Enable non-canonical mode
pub fn set_raw_mode(fd: c_int) -> Result<()> {
    let mut attr = attributes(fd)?;

    // Change flags to enable non-canonical mode
    unsafe {
        libc::cfmakeraw(&mut attr);
    }

    set_attributes(fd, &attr)
}

// Return minimum input queue length and timeout duration for read()
pub fn read_mode(fd: c_int) -> Result<(usize, Duration)> {
    let attr = attributes(fd)?;

    Ok((
        attr.c_cc[VMIN] as usize,
        // Converted from deciseconds
        Duration::from_millis(u64::from(attr.c_cc[VTIME]) * 100),
    ))
}

// Set minimum input queue length and timeout duration for read()
pub fn set_read_mode(fd: c_int, min_length: u8, timeout: Duration) -> Result<()> {
    let mut attr = attributes(fd)?;

    attr.c_cc[VMIN] = min_length;
    // Specified in deciseconds
    attr.c_cc[VTIME] = (timeout.as_secs() * 10)
        .saturating_add(u64::from(timeout.subsec_micros() / 100_000))
        .min(255) as u8;

    set_attributes(fd, &attr)
}

// If CREAD isn't set, all input is discarded
pub fn enable_read(fd: c_int) -> Result<()> {
    let mut attr = attributes(fd)?;
    attr.c_cflag |= CREAD;

    set_attributes(fd, &attr)
}

// Ignore carrier detect signal
pub fn ignore_carrier_detect(fd: c_int) -> Result<()> {
    let mut attr = attributes(fd)?;
    attr.c_cflag |= CLOCAL;

    set_attributes(fd, &attr)
}

// Return RTS/CTS flow control setting
pub fn hardware_flow_control(fd: c_int) -> Result<bool> {
    let attr = attributes(fd)?;

    Ok((attr.c_cflag & CRTSCTS) > 0)
}

// Set RTS/CTS flow control
pub fn set_hardware_flow_control(fd: c_int, enabled: bool) -> Result<()> {
    let mut attr = attributes(fd)?;

    if enabled {
        attr.c_cflag |= CRTSCTS;
    } else {
        attr.c_cflag &= !CRTSCTS;
    }

    set_attributes(fd, &attr)
}

// Return control signal status
pub fn status(fd: c_int) -> Result<c_int> {
    let mut tiocm: c_int = 0;

    parse_retval!(unsafe { libc::ioctl(fd, TIOCMGET, &mut tiocm) })?;

    Ok(tiocm)
}

// Return CTS state
pub fn cts(fd: c_int) -> Result<bool> {
    let mut tiocm: c_int = 0;

    parse_retval!(unsafe { libc::ioctl(fd, TIOCMGET, &mut tiocm) })?;

    Ok(tiocm & TIOCM_CTS > 0)
}

// Return RTS state
pub fn rts(fd: c_int) -> Result<bool> {
    let mut tiocm: c_int = 0;

    parse_retval!(unsafe { libc::ioctl(fd, TIOCMGET, &mut tiocm) })?;

    Ok(tiocm & TIOCM_RTS > 0)
}

// Assert / release RTS line
pub fn set_rts(fd: c_int, rts: bool) -> Result<()> {
    let tiocm: c_int = TIOCM_RTS;

    parse_retval!(unsafe { libc::ioctl(fd, if rts { TIOCMBIS } else { TIOCMBIC }, &tiocm) })?;

    Ok(())
}

// Return DCD state
pub fn dcd(fd: c_int) -> Result<bool> {
    let mut tiocm: c_int = 0;

    parse_retval!(unsafe { libc::ioctl(fd, TIOCMGET, &mut tiocm) })?;

    Ok(tiocm & TIOCM_CAR > 0)
}

// Return RI state
pub fn ri(fd: c_int) -> Result<bool> {
    let mut tiocm: c_int = 0;

    parse_retval!(unsafe { libc::ioctl(fd, TIOCMGET, &mut tiocm) })?;

    Ok(tiocm & TIOCM_RNG > 0)
}

// Return DSR state
pub fn dsr(fd: c_int) -> Result<bool> {
    let mut tiocm: c_int = 0;

    parse_retval!(unsafe { libc::ioctl(fd, TIOCMGET, &mut tiocm) })?;

    Ok(tiocm & TIOCM_DSR > 0)
}

// Return DTR state
pub fn dtr(fd: c_int) -> Result<bool> {
    let mut tiocm: c_int = 0;

    parse_retval!(unsafe { libc::ioctl(fd, TIOCMGET, &mut tiocm) })?;

    Ok(tiocm & TIOCM_DTR > 0)
}

// Assert / release DTR line
pub fn set_dtr(fd: c_int, dtr: bool) -> Result<()> {
    let tiocm: c_int = TIOCM_DTR;

    parse_retval!(unsafe { libc::ioctl(fd, if dtr { TIOCMBIS } else { TIOCMBIC }, &tiocm) })?;

    Ok(())
}

// Return XON/XOFF flow control setting
pub fn software_flow_control(fd: c_int) -> Result<(bool, bool)> {
    let attr = attributes(fd)?;

    Ok(((attr.c_iflag & IXOFF) > 0, (attr.c_iflag & IXON) > 0))
}

// Set XON/XOFF flow control
pub fn set_software_flow_control(
    fd: c_int,
    incoming_enabled: bool,
    outgoing_enabled: bool,
) -> Result<()> {
    let mut attr = attributes(fd)?;

    attr.c_iflag &= !(IXON | IXOFF | IXANY);
    attr.c_cc[VSTART] = XON;
    attr.c_cc[VSTOP] = XOFF;

    if incoming_enabled {
        attr.c_iflag |= IXOFF;
    }

    if outgoing_enabled {
        attr.c_iflag |= IXON;
    }

    set_attributes(fd, &attr)
}

// Send XOFF
pub fn send_stop(fd: c_int) -> Result<()> {
    parse_retval!(unsafe { libc::tcflow(fd, TCIOFF) })?;

    Ok(())
}

// Send XON
pub fn send_start(fd: c_int) -> Result<()> {
    parse_retval!(unsafe { libc::tcflow(fd, TCION) })?;

    Ok(())
}

// Discard all waiting incoming and outgoing data
pub fn flush(fd: c_int, queue_type: Queue) -> Result<()> {
    parse_retval!(unsafe {
        libc::tcflush(
            fd,
            match queue_type {
                Queue::Input => TCIFLUSH,
                Queue::Output => TCOFLUSH,
                Queue::Both => TCIOFLUSH,
            },
        )
    })?;

    Ok(())
}

// Wait until all outgoing data has been transmitted
pub fn drain(fd: c_int) -> Result<()> {
    parse_retval!(unsafe { libc::tcdrain(fd) })?;

    Ok(())
}

// Returns the number of bytes waiting in the input queue.
pub fn input_len(fd: c_int) -> Result<usize> {
    let mut len: c_int = 0;

    parse_retval!(unsafe { libc::ioctl(fd, TIOCINQ, &mut len) })?;

    Ok(len as usize)
}

// Returns the number of bytes waiting in the output queue.
pub fn output_len(fd: c_int) -> Result<usize> {
    let mut len: c_int = 0;

    parse_retval!(unsafe { libc::ioctl(fd, TIOCOUTQ, &mut len) })?;

    Ok(len as usize)
}

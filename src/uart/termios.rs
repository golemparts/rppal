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

#![allow(unused_imports)]

use std::io;

use libc::{c_int, termios};
use libc::{cfgetospeed, cfmakeraw, cfsetispeed, cfsetospeed, tcgetattr, tcsetattr};
use libc::{B0, B110, B134, B150, B200, B300, B50, B75};
use libc::{B1000000, B1152000, B460800, B500000, B576000, B921600};
use libc::{B115200, B19200, B230400, B38400, B57600};
use libc::{B1200, B1800, B2400, B4800, B600, B9600};
use libc::{B1500000, B2000000, B2500000, B3000000, B3500000, B4000000};

use libc::{CLOCAL, CMSPAR, CREAD, CRTSCTS, TCSANOW};
use libc::{CS5, CS6, CS7, CS8, CSIZE, CSTOPB, PARENB, PARODD};
use libc::{VMIN, VTIME};

use uart::{Error, Parity, Result};

fn parse_retval(retval: c_int) -> Result<i32> {
    if retval == -1 {
        Err(Error::Io(io::Error::last_os_error()))
    } else {
        Ok(retval)
    }
}

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

    parse_retval(unsafe { tcgetattr(fd, &mut attr) })?;

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

    parse_retval(unsafe { tcgetattr(fd, &mut attr) })?;

    Ok(attr)
}

pub fn set_attributes(fd: c_int, attr: &termios) -> Result<()> {
    parse_retval(unsafe { tcsetattr(fd, TCSANOW, attr) })?;

    Ok(())
}

pub fn line_speed(fd: c_int) -> Result<u32> {
    Ok(match unsafe { cfgetospeed(&attributes(fd)?) } {
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

    let mut attr = attributes(fd)?;
    parse_retval(unsafe { cfsetispeed(&mut attr, baud) })?;
    parse_retval(unsafe { cfsetospeed(&mut attr, baud) })?;
    set_attributes(fd, &attr)?;

    Ok(())
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

    Err(Error::InvalidValue)
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

    set_attributes(fd, &attr)?;

    Ok(())
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

    set_attributes(fd, &attr)?;

    Ok(())
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

    set_attributes(fd, &attr)?;

    Ok(())
}

pub fn set_raw_mode(fd: c_int) -> Result<()> {
    let mut attr = attributes(fd)?;
    // Change flags to enable non-canonical mode
    unsafe {
        cfmakeraw(&mut attr);
    }
    attr.c_cc[VMIN] = 0; // Don't block read() when there's no waiting data
    attr.c_cc[VTIME] = 0; // No timeout needed
    set_attributes(fd, &attr)?;

    Ok(())
}

// If CREAD isn't set, all input is discarded
pub fn enable_read(fd: c_int) -> Result<()> {
    let mut attr = attributes(fd)?;
    attr.c_cflag |= CREAD;
    set_attributes(fd, &attr)?;

    Ok(())
}

// Ignore carrier detect signal
pub fn ignore_carrier_detect(fd: c_int) -> Result<()> {
    let mut attr = attributes(fd)?;
    attr.c_cflag |= CLOCAL;
    set_attributes(fd, &attr)?;

    Ok(())
}

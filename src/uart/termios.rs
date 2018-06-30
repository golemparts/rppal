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

use std::io;
use std::result;

use libc::{c_int, termios};
use libc::{cfgetospeed, cfsetispeed, cfsetospeed, tcgetattr, tcsetattr};
use libc::{B0, B110, B134, B150, B200, B300, B50, B75};
use libc::{B115200, B19200, B230400, B38400, B57600};
use libc::{B1200, B1800, B2400, B4800, B600, B9600};
use libc::{CMSPAR, CRTSCTS, PARENB, PARODD, TCSANOW};

pub type Result<T> = result::Result<T, io::Error>;

fn parse_retval(retval: c_int) -> Result<i32> {
    if retval == -1 {
        Err(io::Error::last_os_error())
    } else {
        Ok(retval)
    }
}

pub unsafe fn attributes(fd: c_int) -> Result<termios> {
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

    parse_retval(tcgetattr(fd, &mut attr))?;

    Ok(attr)
}

pub unsafe fn set_attributes(fd: c_int, attr: &termios) -> Result<()> {
    parse_retval(tcsetattr(fd, TCSANOW, attr))?;

    Ok(())
}

pub unsafe fn speed(fd: c_int) -> Result<u32> {
    Ok(match cfgetospeed(&attributes(fd)?) {
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
        _ => 0,
    })
}

pub unsafe fn set_speed(fd: c_int, speed: u32) -> Result<()> {
    let baud = match speed {
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
        _ => unimplemented!(),
    };

    let mut attr = attributes(fd)?;
    parse_retval(cfsetispeed(&mut attr, baud))?;
    parse_retval(cfsetospeed(&mut attr, baud))?;
    set_attributes(fd, &attr)?;

    Ok(())
}

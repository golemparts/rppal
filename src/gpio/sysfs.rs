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

use std::fmt;
use std::fmt::{Display, Formatter};
use std::fs;
use std::fs::File;
use std::io;
use std::io::Write;
use std::os::linux::fs::MetadataExt;
use std::result;
use std::thread;
use std::time::Duration;
use std::path::Path;

use users;

use gpio::Trigger;

quick_error! {
    #[derive(Debug)]
/// Errors that can occur while working with sysfs.
    pub enum Error {
/// IO error.
        Io(err: io::Error) { description(err.description()) from() }
    }
}

/// Result type returned from methods that can have `rppal::gpio::interrupt::Error`s.
pub type Result<T> = result::Result<T, Error>;

pub enum Direction {
    In,
    Out,
    Low,
    High,
}

pub fn export(pin: u8) -> Result<()> {
    // Only export if the pin isn't already exported
    if !Path::new(&format!("/sys/class/gpio/gpio{}", pin)).exists() {
        File::create("/sys/class/gpio/export")?.write_fmt(format_args!("{}", pin))?;
    }

    // The symlink created by exporting a pin starts off owned by root:root. There's
    // a short delay before the group is changed to gpio. Since rppal should work for
    // non-root users, we'll wait for max. 1s for the group to change to gpio.
    let gid_gpio = if let Some(group) = users::get_group_by_name("gpio") {
        group.gid()
    } else {
        0
    };

    let mut counter = 0;
    while counter < 20 {
        let meta = fs::metadata(format!("/sys/class/gpio/gpio{}", pin))?;
        if meta.st_gid() == gid_gpio {
            break;
        }

        thread::sleep(Duration::from_millis(50));
        counter += 1;
    }

    Ok(())
}

pub fn unexport(pin: u8) -> Result<()> {
    // Only unexport if the pin is actually exported
    if Path::new(&format!("/sys/class/gpio/gpio{}", pin)).exists() {
        File::create("/sys/class/gpio/unexport")?.write_fmt(format_args!("{}", pin))?;
    }

    Ok(())
}

pub fn set_direction(pin: u8, direction: Direction) -> Result<()> {
    let b_direction: &[u8] = match direction {
        Direction::In => b"in",
        Direction::Out => b"out",
        Direction::Low => b"low",
        Direction::High => b"high",
    };

    File::create(format!("/sys/class/gpio/gpio{}/direction", pin))?.write(b_direction)?;

    Ok(())
}

pub fn set_edge(pin: u8, trigger: Trigger) -> Result<()> {
    let b_trigger: &[u8] = match trigger {
        Trigger::Disabled => b"none",
        Trigger::RisingEdge => b"rising",
        Trigger::FallingEdge => b"falling",
        Trigger::Both => b"both",
    };

    File::create(format!("/sys/class/gpio/gpio{}/edge", pin))?.write(b_trigger)?;

    Ok(())
}

pub fn open_value(pin: u8) -> Result<File> {
    Ok(File::open(format!("/sys/class/gpio/gpio{}/value", pin))?)
}

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

use std::ffi::CString;
use std::fs;
use std::fs::File;
use std::io;
use std::io::Write;
use std::os::linux::fs::MetadataExt;
use std::path::Path;
use std::result;
use std::thread;
use std::time::Duration;

use libc;

/// Result type returned from methods that can have `io::Error`s.
pub type Result<T> = result::Result<T, io::Error>;

// Find group ID for specified group name
fn group_name_to_gid(name: &str) -> Option<u32> {
    if let Ok(name_cstr) = CString::new(name) {
        unsafe {
            let group_ptr = libc::getgrnam(name_cstr.as_ptr());

            if !group_ptr.is_null() {
                return Some((*group_ptr).gr_gid);
            }
        }
    }

    None
}

pub fn export(channel: u8) -> Result<()> {
    // Only export if the channel isn't already exported
    if !Path::new(&format!("/sys/class/pwm/pwmchip0/pwm{}", channel)).exists() {
        File::create("/sys/class/pwm/pwmchip0/export")?.write_fmt(format_args!("{}", channel))?;
    }

    // Wait 500ms max for the group to change to gpio, provided the proper udev rules have
    // been set up and a recent kernel is installed, which avoids running into permission issues
    // where root access is required. This might require manually adding rules, since they don't
    // seem to be part of the latest release yet. The patched /drivers/pwm/sysfs.c was included
    // in raspberrypi-kernel_1.20180417-1 (4.14.34). See: https://github.com/raspberrypi/linux/issues/1983
    let gid_gpio = if let Some(gid) = group_name_to_gid("gpio") {
        gid
    } else {
        0
    };

    // TODO: If we have superuser privileges, don't bother waiting here.

    let mut counter = 0;
    while counter < 10 {
        let meta = fs::metadata(format!("/sys/class/pwm/pwmchip0/pwm{}", channel))?;
        if meta.st_gid() == gid_gpio {
            break;
        }

        thread::sleep(Duration::from_millis(50));
        counter += 1;
    }

    Ok(())
}

pub fn unexport(channel: u8) -> Result<()> {
    // Only unexport if the channel is actually exported
    if Path::new(&format!("/sys/class/pwm/pwmchip0/pwm{}", channel)).exists() {
        File::create("/sys/class/pwm/pwmchip0/unexport")?.write_fmt(format_args!("{}", channel))?;
    }

    Ok(())
}

pub fn period(channel: u8) -> Result<u64> {
    unimplemented!()
}

pub fn set_period(channel: u8, period: u64) -> Result<()> {
    unimplemented!()
}

pub fn duty_cycle(channel: u8) -> Result<u64> {
    unimplemented!()
}

pub fn set_duty_cycle(channel: u8, duty_cycle: u64) -> Result<()> {
    unimplemented!()
}

pub fn polarity(channel: u8) -> Result<String> {
    unimplemented!()
}

pub fn set_polarity(channel: u8, polarity: &str) -> Result<()> {
    unimplemented!()
}

pub fn enabled(channel: u8) -> Result<bool> {
    unimplemented!()
}

pub fn set_enabled(channel: u8, enabled: bool) -> Result<()> {
    Ok(())
}

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

use std::fs;
use std::fs::File;
use std::io;
use std::io::Write;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;
use std::result;
use std::thread;
use std::time::Duration;

use libc;

use linux;
use pwm::Polarity;

/// Result type returned from methods that can have `io::Error`s.
pub type Result<T> = result::Result<T, io::Error>;

// Check file permissions and group ID
fn check_permissions(path: &str, gid: u32) -> bool {
    if let Ok(metadata) = fs::metadata(path) {
        if metadata.permissions().mode() != 0o040_770 && metadata.permissions().mode() != 0o100_770
        {
            return false;
        }

        if metadata.gid() == gid {
            return true;
        }
    }

    false
}

pub fn export(channel: u8) -> Result<()> {
    // Only export if the channel isn't already exported
    if !Path::new(&format!("/sys/class/pwm/pwmchip0/pwm{}", channel)).exists() {
        File::create("/sys/class/pwm/pwmchip0/export")?.write_fmt(format_args!("{}", channel))?;
    }

    // If we're logged in as root or effective root, skip the permission checks
    if let Some(root_uid) = linux::user_to_uid("root") {
        unsafe {
            if libc::getuid() == root_uid || libc::geteuid() == root_uid {
                return Ok(());
            }
        }
    }

    // Wait 1s max for the group to change to gpio, and group permissions to be set,
    // provided the proper udev rules have been set up and a recent kernel is installed, which
    // avoids running into permission issues where root access is required. This might require
    // manually adding rules, since they don't seem to be part of the latest release yet. The
    // patched drivers/pwm/sysfs.c was included in raspberrypi-kernel_1.20180417-1 (4.14.34).
    // See: https://github.com/raspberrypi/linux/issues/1983
    let gid_gpio = if let Some(gid) = linux::group_to_gid("gpio") {
        gid
    } else {
        0
    };

    let paths = &[
        format!("/sys/class/pwm/pwmchip0/pwm{}", channel),
        format!("/sys/class/pwm/pwmchip0/pwm{}/period", channel),
        format!("/sys/class/pwm/pwmchip0/pwm{}/duty_cycle", channel),
        format!("/sys/class/pwm/pwmchip0/pwm{}/polarity", channel),
        format!("/sys/class/pwm/pwmchip0/pwm{}/enable", channel),
    ];

    let mut counter = 0;
    'counter: while counter < 25 {
        for path in paths {
            if !check_permissions(path, gid_gpio) {
                // This should normally be set within the first ~30ms.
                thread::sleep(Duration::from_millis(40));
                counter += 1;

                continue 'counter;
            }
        }

        break;
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
    let period = fs::read_to_string(format!("/sys/class/pwm/pwmchip0/pwm{}/period", channel))?;

    if let Ok(period_u64) = u64::from_str_radix(&period, 10) {
        Ok(period_u64)
    } else {
        Ok(0)
    }
}

pub fn set_period(channel: u8, period: u64) -> Result<()> {
    File::create(format!("/sys/class/pwm/pwmchip0/pwm{}/period", channel))?
        .write_fmt(format_args!("{}", period))?;

    Ok(())
}

pub fn duty_cycle(channel: u8) -> Result<u64> {
    let duty_cycle =
        fs::read_to_string(format!("/sys/class/pwm/pwmchip0/pwm{}/duty_cycle", channel))?;

    if let Ok(duty_cycle_u64) = u64::from_str_radix(&duty_cycle, 10) {
        Ok(duty_cycle_u64)
    } else {
        Ok(0)
    }
}

pub fn set_duty_cycle(channel: u8, duty_cycle: u64) -> Result<()> {
    File::create(format!("/sys/class/pwm/pwmchip0/pwm{}/duty_cycle", channel))?
        .write_fmt(format_args!("{}", duty_cycle))?;

    Ok(())
}

pub fn polarity(channel: u8) -> Result<Polarity> {
    let polarity = fs::read_to_string(format!("/sys/class/pwm/pwmchip0/pwm{}/polarity", channel))?;

    match &polarity[..] {
        "normal" => Ok(Polarity::Normal),
        _ => Ok(Polarity::Inverse),
    }
}

pub fn set_polarity(channel: u8, polarity: Polarity) -> Result<()> {
    let b_polarity: &[u8] = match polarity {
        Polarity::Normal => b"normal",
        Polarity::Inverse => b"inversed",
    };

    File::create(format!("/sys/class/pwm/pwmchip0/pwm{}/polarity", channel))?
        .write_all(b_polarity)?;

    Ok(())
}

pub fn enabled(channel: u8) -> Result<bool> {
    let enabled = fs::read_to_string(format!("/sys/class/pwm/pwmchip0/pwm{}/enable", channel))?;

    match &enabled[..] {
        "0" => Ok(false),
        _ => Ok(true),
    }
}

pub fn set_enabled(channel: u8, enabled: bool) -> Result<()> {
    File::create(format!("/sys/class/pwm/pwmchip0/pwm{}/enable", channel))?
        .write_fmt(format_args!("{}", enabled as u8))?;

    Ok(())
}
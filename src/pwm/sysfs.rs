#![allow(clippy::unnecessary_cast)]

use std::ffi::CString;
use std::fs;
use std::fs::File;
use std::io;
use std::io::Write;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;
use std::ptr;
use std::result;
use std::thread;
use std::time::Duration;

use libc::{c_char, group, passwd};

use crate::pwm::Polarity;

/// Result type returned from methods that can have `io::Error`s.
pub type Result<T> = result::Result<T, io::Error>;

// Find user ID for specified user
pub fn user_to_uid(name: &str) -> Option<u32> {
    if let Ok(name_cstr) = CString::new(name) {
        let buf = &mut [0 as c_char; 4096];
        let mut res: *mut passwd = ptr::null_mut();
        let mut pwd = passwd {
            pw_name: ptr::null_mut(),
            pw_passwd: ptr::null_mut(),
            pw_uid: 0,
            pw_gid: 0,
            pw_gecos: ptr::null_mut(),
            pw_dir: ptr::null_mut(),
            pw_shell: ptr::null_mut(),
        };

        unsafe {
            if libc::getpwnam_r(
                name_cstr.as_ptr(),
                &mut pwd,
                buf.as_mut_ptr(),
                buf.len(),
                &mut res,
            ) == 0
                && res as usize > 0
            {
                return Some((*res).pw_uid);
            }
        }
    }

    None
}

// Find group ID for specified group
pub fn group_to_gid(name: &str) -> Option<u32> {
    if let Ok(name_cstr) = CString::new(name) {
        let buf = &mut [0 as c_char; 4096];
        let mut res: *mut group = ptr::null_mut();
        let mut grp = group {
            gr_name: ptr::null_mut(),
            gr_passwd: ptr::null_mut(),
            gr_gid: 0,
            gr_mem: ptr::null_mut(),
        };

        unsafe {
            if libc::getgrnam_r(
                name_cstr.as_ptr(),
                &mut grp,
                buf.as_mut_ptr(),
                buf.len(),
                &mut res,
            ) == 0
                && res as usize > 0
            {
                return Some((*res).gr_gid);
            }
        }
    }

    None
}

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

pub fn export(chip: u8, channel: u8) -> Result<()> {
    // Only export if the channel isn't already exported
    if !Path::new(&format!("/sys/class/pwm/pwmchip{}/pwm{}", chip, channel)).exists() {
        File::create(format!("/sys/class/pwm/pwmchip{}/export", chip))?
            .write_fmt(format_args!("{}", channel))?;
    }

    // If we're logged in as root or effective root, skip the permission checks
    if let Some(root_uid) = user_to_uid("root") {
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
    let gid_gpio = if let Some(gid) = group_to_gid("gpio") {
        gid
    } else {
        0
    };

    let paths = &[
        format!("/sys/class/pwm/pwmchip{}/pwm{}", chip, channel),
        format!("/sys/class/pwm/pwmchip{}/pwm{}/period", chip, channel),
        format!("/sys/class/pwm/pwmchip{}/pwm{}/duty_cycle", chip, channel),
        format!("/sys/class/pwm/pwmchip{}/pwm{}/polarity", chip, channel),
        format!("/sys/class/pwm/pwmchip{}/pwm{}/enable", chip, channel),
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

pub fn unexport(chip: u8, channel: u8) -> Result<()> {
    // Only unexport if the channel is actually exported
    if Path::new(&format!("/sys/class/pwm/pwmchip{}/pwm{}", chip, channel)).exists() {
        File::create(format!("/sys/class/pwm/pwmchip{}/unexport", chip))?
            .write_fmt(format_args!("{}", channel))?;
    }

    Ok(())
}

pub fn period(chip: u8, channel: u8) -> Result<u64> {
    let period = fs::read_to_string(format!(
        "/sys/class/pwm/pwmchip{}/pwm{}/period",
        chip, channel
    ))?;
    if let Ok(period) = period.trim().parse() {
        Ok(period)
    } else {
        Ok(0)
    }
}

pub fn set_period(chip: u8, channel: u8, period: u64) -> Result<()> {
    File::create(format!(
        "/sys/class/pwm/pwmchip{}/pwm{}/period",
        chip, channel
    ))?
    .write_fmt(format_args!("{}", period))?;

    Ok(())
}

pub fn pulse_width(chip: u8, channel: u8) -> Result<u64> {
    // The sysfs PWM interface specifies the duty cycle in nanoseconds, which
    // means it's actually the pulse width.
    let duty_cycle = fs::read_to_string(format!(
        "/sys/class/pwm/pwmchip{}/pwm{}/duty_cycle",
        chip, channel
    ))?;

    if let Ok(duty_cycle) = duty_cycle.trim().parse() {
        Ok(duty_cycle)
    } else {
        Ok(0)
    }
}

pub fn set_pulse_width(chip: u8, channel: u8, pulse_width: u64) -> Result<()> {
    // The sysfs PWM interface specifies the duty cycle in nanoseconds, which
    // means it's actually the pulse width.
    File::create(format!(
        "/sys/class/pwm/pwmchip{}/pwm{}/duty_cycle",
        chip, channel
    ))?
    .write_fmt(format_args!("{}", pulse_width))?;

    Ok(())
}

pub fn polarity(chip: u8, channel: u8) -> Result<Polarity> {
    let polarity = fs::read_to_string(format!(
        "/sys/class/pwm/pwmchip{}/pwm{}/polarity",
        chip, channel
    ))?;

    match polarity.trim() {
        "normal" => Ok(Polarity::Normal),
        _ => Ok(Polarity::Inverse),
    }
}

pub fn set_polarity(chip: u8, channel: u8, polarity: Polarity) -> Result<()> {
    let b_polarity: &[u8] = match polarity {
        Polarity::Normal => b"normal",
        Polarity::Inverse => b"inversed",
    };

    File::create(format!(
        "/sys/class/pwm/pwmchip{}/pwm{}/polarity",
        chip, channel
    ))?
    .write_all(b_polarity)?;

    Ok(())
}

pub fn enabled(chip: u8, channel: u8) -> Result<bool> {
    let enabled = fs::read_to_string(format!(
        "/sys/class/pwm/pwmchip{}/pwm{}/enable",
        chip, channel
    ))?;

    match enabled.trim() {
        "0" => Ok(false),
        _ => Ok(true),
    }
}

pub fn set_enabled(chip: u8, channel: u8, enabled: bool) -> Result<()> {
    File::create(format!(
        "/sys/class/pwm/pwmchip{}/pwm{}/enable",
        chip, channel
    ))?
    .write_fmt(format_args!("{}", enabled as u8))
    .map_err(|e| {
        if e.kind() == io::ErrorKind::InvalidInput {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "Make sure you have set either a period or frequency before enabling PWM",
            )
        } else {
            e
        }
    })?;

    Ok(())
}

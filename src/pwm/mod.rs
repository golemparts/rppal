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

//! Interface for the PWM peripheral.
//!
//! ## PWM channels
//!
//!
//!
//! ## Using PWM without superuser privileges (`sudo`)
//!
//! As of kernel version 4.14.34, released on April 16 2018, it's possible to
//! configure your Raspberry Pi to allow non-root access to PWM. 4.14.34 includes
//! a [patch] that allows udev to change file permissions when a
//! PWM channel is exported. This will let any user that's a member of the `gpio`
//! group configure PWM without having to use `sudo`.
//!
//! The udev rules needed to make this work haven't been patched in yet as of
//! June 2018, but you can easily add them yourself. Make sure you're running
//! 4.14.34 or later, and append the following snippet to
//! `/etc/udev/rules.d/99-com.rules`. Reboot the Raspberry Pi afterwards.
//!
//! ```text
//! SUBSYSTEM=="pwm*", PROGRAM="/bin/sh -c '\
//!     chown -R root:gpio /sys/class/pwm && chmod -R 770 /sys/class/pwm;\
//!     chown -R root:gpio /sys/devices/platform/soc/*.pwm/pwm/pwmchip* &&\
//!     chmod -R 770 /sys/devices/platform/soc/*.pwm/pwm/pwmchip*\
//! '"
//! ```
//!
//! ## Troubleshooting
//!
//! ### Permission Denied
//!
//! [patch]: https://github.com/raspberrypi/linux/issues/1983

pub enum Channel {
    Pwm0,
    Pwm1,
}

pub enum Polarity {
    Normal,
    Inverse,
}

pub struct Pwm {
    channel: Channel,
}

impl Pwm {
    pub fn new(channel: Channel) -> Pwm {
        Pwm { channel }
    }
}

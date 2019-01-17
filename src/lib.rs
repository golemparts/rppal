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

//! RPPAL is a Rust library that provides access to the Raspberry Pi's GPIO,
//! I2C, PWM and SPI peripherals. Support for additional peripherals will be
//! added in future updates. The library is compatible with the Raspberry Pi A,
//! A+, B, B+, 2B, 3A+, 3B, 3B+, Compute, Compute 3, Zero and Zero W.

// Used by rustdoc to link other crates to rppal's docs
#![doc(html_root_url = "https://docs.rs/rppal/0.10.0")]

mod user;

#[macro_use]
mod macros;

pub mod gpio;
pub mod i2c;
pub mod pwm;
pub mod spi;
pub mod system;
// pub mod uart;

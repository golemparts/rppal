// Copyright (c) 2017 Rene van der Meer
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

//! A Rust library that provides access to the Raspberry Pi GPIO peripheral
//! through either `/dev/gpiomem` or `/dev/mem`. Support for additional peripherals,
//! as well as useful helper functions, will be added in future updates.
//!
//! The library is compatible with the BCM2835, BCM2836 and BCM2837 SoCs.

#![recursion_limit = "128"] // Needed for the quick_error! macro

#[macro_use]
extern crate enum_primitive;
extern crate libc;
extern crate num;
#[macro_use]
extern crate quick_error;

pub mod gpio;
pub mod system;

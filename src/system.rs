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

//! Raspberry Pi system-related tools.
//!
//! Use `DeviceInfo` to identify what Raspberry Pi model and SoC the software is
//! running on. This information is used internally to calculate the correct memory
//! locations for the various BCM283x peripherals.

use std::fmt;
use std::fs::File;
use std::io::{BufReader, BufRead};
use std::result;

const BCM2708_PERIPHERAL_BASE: u32 = 0x20000000;
const BCM2709_PERIPHERAL_BASE: u32 = 0x3f000000;
const BCM2710_PERIPHERAL_BASE: u32 = 0x3f000000;
const GPIO_OFFSET: u32 = 0x200000;

quick_error! {
    #[derive(Debug)]
/// Errors that can occur when trying to identify the Raspberry Pi hardware.
    pub enum Error {
/// Unknown SoC.
///
/// Based on the output of `/proc/cpuinfo`, it wasn't possible to identify the SoC used by the
/// Raspberry Pi. While running the library on an unknown Raspberry Pi model is acceptable,
/// identifying the SoC is required because the memory address for the GPIO peripheral
/// depends on it.
        UnknownSoC { description("unknown SoC") }
/// Can't access `/proc/cpuinfo`.
///
/// Unable to read the contents of `/proc/cpuinfo`. This could be an issue with permissions, or
/// a Linux distribution is used that doesn't provide access to this virtual file.
        CantAccessProcCPUInfo { description("can't access /proc/cpuinfo") }
    }
}

/// Result type returned from methods that can have `system::Error`s.
pub type Result<T> = result::Result<T, Error>;

#[derive(Debug, PartialEq, Copy, Clone)]
/// Identifiable Raspberry Pi models.
pub enum Model {
    RaspberryPiA,
    RaspberryPiAPlus,
    RaspberryPiB,
    RaspberryPiBPlus,
    RaspberryPi2B,
    RaspberryPi3B,
    RaspberryPiComputeModule,
    RaspberryPiComputeModule3,
    RaspberryPiZero,
    RaspberryPiZeroW,
    Unknown,
}

impl fmt::Display for Model {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Model::RaspberryPiA => write!(f, "Raspberry Pi A"),
            Model::RaspberryPiAPlus => write!(f, "Raspberry Pi A+"),
            Model::RaspberryPiB => write!(f, "Raspberry Pi B"),
            Model::RaspberryPiBPlus => write!(f, "Raspberry Pi B+"),
            Model::RaspberryPi2B => write!(f, "Raspberry Pi 2 B"),
            Model::RaspberryPi3B => write!(f, "Raspberry Pi 3 B"),
            Model::RaspberryPiComputeModule => write!(f, "Raspberry Pi Compute Module"),
            Model::RaspberryPiComputeModule3 => write!(f, "Raspberry Pi Compute Module 3"),
            Model::RaspberryPiZero => write!(f, "Raspberry Pi Zero"),
            Model::RaspberryPiZeroW => write!(f, "Raspberry Pi Zero W"),
            Model::Unknown => write!(f, "Unknown"),
        }
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
/// Identifiable Raspberry Pi SoCs.
pub enum SoC {
    BCM2835,
    BCM2836,
    BCM2837,
}

impl fmt::Display for SoC {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            SoC::BCM2835 => write!(f, "BCM2835"),
            SoC::BCM2836 => write!(f, "BCM2836"),
            SoC::BCM2837 => write!(f, "BCM2837"),
        }
    }
}

/// Retrieves Raspberry Pi device information.
pub struct DeviceInfo {
    model: Model,
    soc: SoC,
    peripheral_base: u32,
    gpio_offset: u32,
}

impl DeviceInfo {
    /// Constructs a new `DeviceInfo`.
    pub fn new() -> Result<DeviceInfo> {
        // Parse hardware/revision from /proc/cpuinfo to figure out model/SoC
        let proc_cpuinfo = BufReader::new(match File::open("/proc/cpuinfo") {
            Err(_) => return Err(Error::CantAccessProcCPUInfo),
            Ok(file) => file,
        });

        let mut hardware: String = String::new();
        let mut revision: String = String::new();
        for line_result in proc_cpuinfo.lines() {
            if let Some(line) = line_result.ok() {
                if line.starts_with("Hardware\t: ") {
                    hardware = String::from(&line[11..]);
                } else if line.starts_with("Revision\t: ") {
                    revision = String::from(&line[11..]).to_lowercase();
                }
            }
        }

        let model = if revision.len() >= 6 {
            // Newer revisions consist of at least 6 characters
            match &revision[revision.len() - 3..revision.len() - 1] {
                "00" => Model::RaspberryPiA,
                "01" => Model::RaspberryPiB,
                "02" => Model::RaspberryPiAPlus,
                "03" => Model::RaspberryPiBPlus,
                "04" => Model::RaspberryPi2B,
                "06" => Model::RaspberryPiComputeModule,
                "08" => Model::RaspberryPi3B,
                "09" => Model::RaspberryPiZero,
                "0a" => Model::RaspberryPiComputeModule3,
                "0c" => Model::RaspberryPiZeroW,
                _ => Model::Unknown,
            }
        } else if revision.len() == 4 {
            // Older revisions are 4 characters long
            match &revision[..] {
                "0007" | "0008" | "0009" => Model::RaspberryPiA,
                "0002" | "0003" | "0004" | "0005" | "0006" | "000d" | "000e" | "000f" => {
                    Model::RaspberryPiB
                }
                "0012" => Model::RaspberryPiAPlus,
                "0010" | "0013" => Model::RaspberryPiBPlus,
                "0011" => Model::RaspberryPiComputeModule,
                _ => Model::Unknown,
            }
        } else {
            Model::Unknown
        };

        // Make sure we're actually running on a supported SoC
        match &hardware[..] {
            "BCM2708" | "BCM2835" => {
                Ok(DeviceInfo {
                    model: model,
                    soc: SoC::BCM2835,
                    peripheral_base: BCM2708_PERIPHERAL_BASE,
                    gpio_offset: GPIO_OFFSET,
                })
            }
            "BCM2709" | "BCM2836" => {
                Ok(DeviceInfo {
                    model: model,
                    soc: SoC::BCM2836,
                    peripheral_base: BCM2709_PERIPHERAL_BASE,
                    gpio_offset: GPIO_OFFSET,
                })
            }
            "BCM2710" | "BCM2837" => {
                Ok(DeviceInfo {
                    model: model,
                    soc: SoC::BCM2837,
                    peripheral_base: BCM2710_PERIPHERAL_BASE,
                    gpio_offset: GPIO_OFFSET,
                })
            }
            _ => return Err(Error::UnknownSoC),
        }
    }

    /// Returns a reference to the Raspberry Pi model identified by parsing the contents of `/proc/cpuinfo`.
    pub fn model(&self) -> &Model {
        &self.model
    }

    /// Returns a reference to the SoC identified by parsing the contents of `/proc/cpuinfo`.
    pub fn soc(&self) -> &SoC {
        &self.soc
    }

    /// Returns the base memory address for the BCM283x peripherals.
    pub fn peripheral_base(&self) -> u32 {
        self.peripheral_base
    }

    /// Returns the offset memory address for the GPIO section.
    pub fn gpio_offset(&self) -> u32 {
        self.gpio_offset
    }
}

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

//! Raspberry Pi system-related tools.
//!
//! Use [`DeviceInfo`] to identify the Raspberry Pi's model and SoC.
//!
//! [`DeviceInfo`]: struct.DeviceInfo.html

use std::error;
use std::fmt;
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::result;

const PERIPHERAL_BASE_RPI: u32 = 0x2000_0000;
const PERIPHERAL_BASE_RPI2: u32 = 0x3f00_0000;
const GPIO_OFFSET: u32 = 0x20_0000;

/// Errors that can occur when trying to identify the Raspberry Pi hardware.
#[derive(Debug)]
pub enum Error {
    /// Unknown model.
    ///
    /// `DeviceInfo` was unable to identify the Raspberry Pi model based on the
    /// contents of `/proc/cpuinfo`, `/sys/firmware/devicetree/base/compatible`
    /// and `/sys/firmware/devicetree/base/model`.
    ///
    /// Support for new models is usually added shortly after they are officially
    /// announced and available to the public. Make sure you're using the latest
    /// release of RPPAL.
    ///
    /// You may also encounter this error if your Linux distribution
    /// doesn't provide any of the common user-accessible system files
    /// that are used to identify the model and SoC.
    UnknownModel,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Error::UnknownModel => write!(f, "Unknown Raspberry Pi model"),
        }
    }
}

impl error::Error for Error {}

/// Result type returned from methods that can have `system::Error`s.
pub type Result<T> = result::Result<T, Error>;

/// Identifiable Raspberry Pi models.
///
/// `Model` might be extended with additional variants in a minor or
/// patch revision, and must not be exhaustively matched against.
/// Instead, add a `_` catch-all arm to match future variants.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Model {
    RaspberryPiA,
    RaspberryPiAPlus,
    RaspberryPiBRev1,
    RaspberryPiBRev2,
    RaspberryPiBPlus,
    RaspberryPi2B,
    RaspberryPi3APlus,
    RaspberryPi3B,
    RaspberryPi3BPlus,
    RaspberryPi4B,
    RaspberryPiComputeModule,
    RaspberryPiComputeModule3,
    RaspberryPiComputeModule3Plus,
    RaspberryPiZero,
    RaspberryPiZeroW,
    /// `Model` might be extended with additional variants in a minor or
    /// patch revision, and must not be exhaustively matched against.
    /// Instead, add a `_` catch-all arm to match future variants.
    #[doc(hidden)]
    __Nonexhaustive,
}

impl fmt::Display for Model {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Model::RaspberryPiA => write!(f, "Raspberry Pi A"),
            Model::RaspberryPiAPlus => write!(f, "Raspberry Pi A+"),
            Model::RaspberryPiBRev1 => write!(f, "Raspberry Pi B Rev 1"),
            Model::RaspberryPiBRev2 => write!(f, "Raspberry Pi B Rev 2"),
            Model::RaspberryPiBPlus => write!(f, "Raspberry Pi B+"),
            Model::RaspberryPi2B => write!(f, "Raspberry Pi 2 B"),
            Model::RaspberryPi3B => write!(f, "Raspberry Pi 3 B"),
            Model::RaspberryPi3BPlus => write!(f, "Raspberry Pi 3 B+"),
            Model::RaspberryPi3APlus => write!(f, "Raspberry Pi 3 A+"),
            Model::RaspberryPi4B => write!(f, "Raspberry Pi 4 B"),
            Model::RaspberryPiComputeModule => write!(f, "Raspberry Pi Compute Module"),
            Model::RaspberryPiComputeModule3 => write!(f, "Raspberry Pi Compute Module 3"),
            Model::RaspberryPiComputeModule3Plus => write!(f, "Raspberry Pi Compute Module 3+"),
            Model::RaspberryPiZero => write!(f, "Raspberry Pi Zero"),
            Model::RaspberryPiZeroW => write!(f, "Raspberry Pi Zero W"),
            Model::__Nonexhaustive => write!(f, "__Nonexhaustive"),
        }
    }
}

/// Identifiable Raspberry Pi SoCs.
///
/// `SoC` might be extended with additional variants in a minor or
/// patch revision, and must not be exhaustively matched against.
/// Instead, add a `_` catch-all arm to match future variants.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum SoC {
    Bcm2835,
    Bcm2836,
    Bcm2837A1,
    Bcm2837B0,
    Bcm2711,
    /// `SoC` might be extended with additional variants in a minor or
    /// patch revision, and must not be exhaustively matched against.
    /// Instead, add a `_` catch-all arm to match future variants.
    #[doc(hidden)]
    __Nonexhaustive,
}

impl fmt::Display for SoC {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            SoC::Bcm2835 => write!(f, "BCM2835"),
            SoC::Bcm2836 => write!(f, "BCM2836"),
            SoC::Bcm2837A1 => write!(f, "BCM2837A1"),
            SoC::Bcm2837B0 => write!(f, "BCM2837B0"),
            SoC::Bcm2711 => write!(f, "BCM2711"),
            SoC::__Nonexhaustive => write!(f, "__Nonexhaustive"),
        }
    }
}

// Identify Pi model based on /proc/cpuinfo
fn parse_proc_cpuinfo() -> Result<Model> {
    let proc_cpuinfo = BufReader::new(match File::open("/proc/cpuinfo") {
        Ok(file) => file,
        Err(_) => return Err(Error::UnknownModel),
    });

    let mut hardware: String = String::new();
    let mut revision: String = String::new();
    for line_result in proc_cpuinfo.lines() {
        if let Ok(line) = line_result {
            if line.starts_with("Hardware\t: ") {
                hardware = String::from(&line[11..]);
            } else if line.starts_with("Revision\t: ") {
                revision = String::from(&line[11..]).to_lowercase();
            }
        }
    }

    // Return an error if we don't recognize the SoC. This check is
    // done to prevent accidentally identifying a non-Pi SBC as a Pi
    // solely based on the revision field.
    match &hardware[..] {
        "BCM2708" | "BCM2835" | "BCM2709" | "BCM2836" | "BCM2710" | "BCM2837" | "BCM2837A1"
        | "BCM2837B0" | "BCM2711" => {}
        _ => return Err(Error::UnknownModel),
    }

    let model = if (revision.len() == 4) || (revision.len() == 8) {
        // Older revisions are 4 characters long, or 8 if they've been over-volted
        match &revision[revision.len() - 4..] {
            "0007" | "0008" | "0009" | "0015" => Model::RaspberryPiA,
            "beta" | "0002" | "0003" => Model::RaspberryPiBRev1,
            "0004" | "0005" | "0006" | "000d" | "000e" | "000f" => Model::RaspberryPiBRev2,
            "0012" => Model::RaspberryPiAPlus,
            "0010" | "0013" => Model::RaspberryPiBPlus,
            "0011" | "0014" => Model::RaspberryPiComputeModule,
            _ => return Err(Error::UnknownModel),
        }
    } else if revision.len() >= 6 {
        // Newer revisions consist of at least 6 characters
        match &revision[..] {
            "900021" => Model::RaspberryPiAPlus,
            "900032" => Model::RaspberryPiBPlus,
            "a01040" | "a01041" | "a21041" | "a22042" => Model::RaspberryPi2B,
            "a02082" | "a22082" | "a32082" | "a52082" => Model::RaspberryPi3B,
            "900092" | "900093" | "920092" | "920093" => Model::RaspberryPiZero,
            "a020a0" | "a220a0" => Model::RaspberryPiComputeModule3,
            "9000c1" => Model::RaspberryPiZeroW,
            "a020d3" => Model::RaspberryPi3BPlus,
            "9020e0" => Model::RaspberryPi3APlus,
            "a02100" => Model::RaspberryPiComputeModule3Plus,
            "a03111" | "b03111" | "c03111" => Model::RaspberryPi4B,
            _ => return Err(Error::UnknownModel),
        }
    } else {
        return Err(Error::UnknownModel);
    };

    Ok(model)
}

// Identify Pi model based on /sys/firmware/devicetree/base/compatible
fn parse_base_compatible() -> Result<Model> {
    let base_compatible = match fs::read_to_string("/sys/firmware/devicetree/base/compatible") {
        Ok(buffer) => buffer,
        Err(_) => return Err(Error::UnknownModel),
    };

    // Based on /arch/arm/boot/dts/ and /Documentation/devicetree/bindings/arm/bcm/
    for comp_id in base_compatible.split('\0') {
        let model = match comp_id {
            "raspberrypi,model-b-i2c0" => Model::RaspberryPiBRev1,
            "raspberrypi,model-b" => Model::RaspberryPiBRev1,
            "raspberrypi,model-a" => Model::RaspberryPiA,
            "raspberrypi,model-b-rev2" => Model::RaspberryPiBRev2,
            "raspberrypi,model-a-plus" => Model::RaspberryPiAPlus,
            "raspberrypi,model-b-plus" => Model::RaspberryPiBPlus,
            "raspberrypi,2-model-b" => Model::RaspberryPi2B,
            "raspberrypi,compute-module" => Model::RaspberryPiComputeModule,
            "raspberrypi,3-model-b" => Model::RaspberryPi3B,
            "raspberrypi,model-zero" => Model::RaspberryPiZero,
            "raspberrypi,3-compute-module" => Model::RaspberryPiComputeModule3,
            "raspberrypi,3-compute-module-plus" => Model::RaspberryPiComputeModule3Plus,
            "raspberrypi,model-zero-w" => Model::RaspberryPiZeroW,
            "raspberrypi,3-model-b-plus" => Model::RaspberryPi3BPlus,
            "raspberrypi,3-model-a-plus" => Model::RaspberryPi3APlus,
            "raspberrypi,4-model-b" => Model::RaspberryPi4B,
            _ => continue,
        };

        return Ok(model);
    }

    Err(Error::UnknownModel)
}

// Identify Pi model based on /sys/firmware/devicetree/base/model
fn parse_base_model() -> Result<Model> {
    let mut base_model = match fs::read_to_string("/sys/firmware/devicetree/base/model") {
        Ok(mut buffer) => {
            if let Some(idx) = buffer.find('\0') {
                buffer.truncate(idx);
                buffer
            } else {
                buffer
            }
        }
        Err(_) => return Err(Error::UnknownModel),
    };

    // Check if this is a Pi B rev 2 before we remove the revision part, assuming the
    // PCB Revision numbers on https://elinux.org/RPi_HardwareHistory are correct, and
    // the installed distro appends the revision to the model name.
    match &base_model[..] {
        "Raspberry Pi Model B Rev 2.0" => return Ok(Model::RaspberryPiBRev2),
        "Raspberry Pi Model B rev2 Rev 2.0" => return Ok(Model::RaspberryPiBRev2),
        _ => (),
    }

    if let Some(idx) = base_model.find(" Rev ") {
        base_model.truncate(idx);
    }

    // Based on /arch/arm/boot/dts/ and /Documentation/devicetree/bindings/arm/bcm/
    let model = match &base_model[..] {
        "Raspberry Pi Model B (no P5)" => Model::RaspberryPiBRev1,
        "Raspberry Pi Model B" => Model::RaspberryPiBRev1,
        "Raspberry Pi Model A" => Model::RaspberryPiA,
        "Raspberry Pi Model B rev2" => Model::RaspberryPiBRev2,
        "Raspberry Pi Model A+" => Model::RaspberryPiAPlus,
        "Raspberry Pi Model A Plus" => Model::RaspberryPiAPlus,
        "Raspberry Pi Model B+" => Model::RaspberryPiBPlus,
        "Raspberry Pi Model B Plus" => Model::RaspberryPiBPlus,
        "Raspberry Pi 2 Model B" => Model::RaspberryPi2B,
        "Raspberry Pi Compute Module" => Model::RaspberryPiComputeModule,
        "Raspberry Pi 3 Model B" => Model::RaspberryPi3B,
        "Raspberry Pi Zero" => Model::RaspberryPiZero,
        "Raspberry Pi Compute Module 3" => Model::RaspberryPiComputeModule3,
        "Raspberry Pi Compute Module 3 Plus" => Model::RaspberryPiComputeModule3Plus,
        "Raspberry Pi Zero W" => Model::RaspberryPiZeroW,
        "Raspberry Pi 3 Model B+" => Model::RaspberryPi3BPlus,
        "Raspberry Pi 3 Model B Plus" => Model::RaspberryPi3BPlus,
        "Raspberry Pi 3 Model A Plus" => Model::RaspberryPi3APlus,
        "Raspberry Pi 4 Model B" => Model::RaspberryPi4B,
        _ => return Err(Error::UnknownModel),
    };

    Ok(model)
}

/// Retrieves Raspberry Pi device information.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct DeviceInfo {
    model: Model,
    soc: SoC,
    peripheral_base: u32,
    gpio_offset: u32,
}

impl DeviceInfo {
    /// Constructs a new `DeviceInfo`.
    ///
    /// `new` attempts to identify the Raspberry Pi's model and SoC based on
    /// the contents of `/proc/cpuinfo`, `/sys/firmware/devicetree/base/compatible`
    /// and `/sys/firmware/devicetree/base/model`.
    pub fn new() -> Result<DeviceInfo> {
        // Parse order from most-detailed to least-detailed info
        let model = parse_proc_cpuinfo()
            .or_else(|_| parse_base_compatible().or_else(|_| parse_base_model()))?;

        // Set SoC and memory offsets based on model
        match model {
            Model::RaspberryPiA
            | Model::RaspberryPiAPlus
            | Model::RaspberryPiBRev1
            | Model::RaspberryPiBRev2
            | Model::RaspberryPiBPlus
            | Model::RaspberryPiComputeModule
            | Model::RaspberryPiZero
            | Model::RaspberryPiZeroW => Ok(DeviceInfo {
                model,
                soc: SoC::Bcm2835,
                peripheral_base: PERIPHERAL_BASE_RPI,
                gpio_offset: GPIO_OFFSET,
            }),
            Model::RaspberryPi2B => Ok(DeviceInfo {
                model,
                soc: SoC::Bcm2836,
                peripheral_base: PERIPHERAL_BASE_RPI2,
                gpio_offset: GPIO_OFFSET,
            }),
            Model::RaspberryPi3B | Model::RaspberryPiComputeModule3 => Ok(DeviceInfo {
                model,
                soc: SoC::Bcm2837A1,
                peripheral_base: PERIPHERAL_BASE_RPI2,
                gpio_offset: GPIO_OFFSET,
            }),
            Model::RaspberryPi3BPlus
            | Model::RaspberryPi3APlus
            | Model::RaspberryPiComputeModule3Plus => Ok(DeviceInfo {
                model,
                soc: SoC::Bcm2837B0,
                peripheral_base: PERIPHERAL_BASE_RPI2,
                gpio_offset: GPIO_OFFSET,
            }),
            Model::RaspberryPi4B => Ok(DeviceInfo {
                model,
                soc: SoC::Bcm2711,
                peripheral_base: PERIPHERAL_BASE_RPI2,
                gpio_offset: GPIO_OFFSET,
            }),
            Model::__Nonexhaustive => unreachable!(),
        }
    }

    /// Returns the Raspberry Pi's model.
    pub fn model(&self) -> Model {
        self.model
    }

    /// Returns the Raspberry Pi's SoC.
    pub fn soc(&self) -> SoC {
        self.soc
    }

    /// Returns the base memory address for the BCM283x peripherals.
    pub(crate) fn peripheral_base(&self) -> u32 {
        self.peripheral_base
    }

    /// Returns the offset from the base memory address for the GPIO section.
    pub(crate) fn gpio_offset(&self) -> u32 {
        self.gpio_offset
    }
}

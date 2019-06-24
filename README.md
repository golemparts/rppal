# RPPAL - Raspberry Pi Peripheral Access Library

[![Build Status](https://travis-ci.com/golemparts/rppal.svg?branch=master)](https://travis-ci.com/golemparts/rppal)
[![crates.io](https://meritbadge.herokuapp.com/rppal)](https://crates.io/crates/rppal)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Minimum rustc version](https://img.shields.io/badge/rustc-v1.31.0-lightgray.svg)](https://blog.rust-lang.org/2018/12/06/Rust-1.31-and-rust-2018.html)

RPPAL provides access to the Raspberry Pi's GPIO, I2C, PWM, SPI and UART peripherals through a user-friendly interface. In addition to peripheral access, RPPAL also offers support for USB to serial adapters. The library can be used in conjunction with a variety of platform-agnostic drivers through its `embedded-hal` trait implementations.

RPPAL requires Raspbian or any similar, recent, Linux distribution. Both `gnu` and `musl` libc targets are supported. RPPAL is compatible with the Raspberry Pi A, A+, B, B+, 2B, 3A+, 3B, 3B+, 4B, CM, CM 3, CM 3+, Zero and Zero W. Backwards compatibility for minor revisions isn't guaranteed until v1.0.0.

This library is under active development on the [master branch](https://github.com/golemparts/rppal/tree/master) of the repository on GitHub. If you're looking for the `README.md` or the `examples` directory for the latest release or any of the earlier releases, visit [crates.io](https://crates.io/crates/rppal), download an archived release from the GitHub [releases](https://github.com/golemparts/rppal/releases) page, or clone and checkout the relevant release tag.

## Table of contents

- [Documentation](#documentation)
- [Usage](#usage)
- [Examples](#examples)
- [Optional features](#optional-features)
- [Supported peripherals](#supported-peripherals)
  - [GPIO](#gpio)
  - [I2C](#i2c)
  - [PWM](#pwm)
  - [SPI](#spi)
  - [UART](#uart)
- [Cross compilation](#cross-compilation)
  - [Cargo](#cargo)
  - [RLS](#rls)
- [Caution](#caution)
- [Copyright and license](#copyright-and-license)

## Documentation

Online documentation is available for the latest release, older releases, and the version currently in development.

* Latest release: [docs.golemparts.com/rppal](https://docs.golemparts.com/rppal)
* Older releases: [docs.rs/rppal](https://docs.rs/rppal)
* In development: [docs.golemparts.com/rppal-dev](https://docs.golemparts.com/rppal-dev)

## Usage

Add a dependency for `rppal` to your `Cargo.toml`.

```toml
[dependencies]
rppal = "0.11.3"
```

If your project requires `embedded-hal` trait implementations, specify either the `hal` or `hal-unproven` feature flag in the dependency declaration.

```toml
[dependencies]
rppal = { version = "0.11.3", features = ["hal"] }
```

Call `new()` on any of the peripherals to construct a new instance.

```rust
use rppal::gpio::Gpio;
use rppal::i2c::I2c;
use rppal::pwm::{Channel, Pwm};
use rppal::spi::{Bus, Mode, SlaveSelect, Spi};
use rppal::uart::{Parity, Uart};

let gpio = Gpio::new()?;
let i2c = I2c::new()?;
let pwm = Pwm::new(Channel::Pwm0)?;
let spi = Spi::new(Bus::Spi0, SlaveSelect::Ss0, 16_000_000, Mode::Mode0)?;
let uart = Uart::new(115_200, Parity::None, 8, 1)?;
```

Access to some peripherals may need to be enabled first through `sudo raspi-config` or by editing `/boot/config.txt`. Refer to the relevant module's documentation for any required steps.

## Examples

This example demonstrates how to blink an LED connected to a GPIO pin. Remember to add a resistor of an appropriate value in series, to prevent exceeding the maximum current rating of the GPIO pin and the LED.

```rust
use std::error::Error;
use std::thread;
use std::time::Duration;

use rppal::gpio::Gpio;
use rppal::system::DeviceInfo;

// Gpio uses BCM pin numbering. BCM GPIO 23 is tied to physical pin 16.
const GPIO_LED: u8 = 23;

fn main() -> Result<(), Box<dyn Error>> {
    println!("Blinking an LED on a {}.", DeviceInfo::new()?.model());

    let mut pin = Gpio::new()?.get(GPIO_LED)?.into_output();

    // Blink the LED by setting the pin's logic level high for 500 ms.
    pin.set_high();
    thread::sleep(Duration::from_millis(500));
    pin.set_low();

    Ok(())
}
```

Additional examples can be found in the `examples` directory.

## Optional features

By default, all optional features are disabled. You can enable a feature by specifying the relevant feature flag(s) in the dependency declaration for `rppal` in your `Cargo.toml`.

* `hal` - Enables `embedded-hal` trait implementations for all supported peripherals. This doesn't include `unproven` traits.
* `hal-unproven` - Enables `embedded-hal` trait implementations for all supported peripherals, including traits marked as `unproven`. Note that `embedded-hal`'s `unproven` traits don't follow semver rules. Patch releases may introduce breaking changes.

## Supported peripherals

### [GPIO](https://docs.golemparts.com/rppal/latest/gpio)

To ensure fast performance, RPPAL controls the GPIO peripheral by directly accessing the registers through either `/dev/gpiomem` or `/dev/mem`. GPIO interrupts are configured using the `gpiochip` character device.

#### Features

* Get/set pin mode and logic level
* Configure built-in pull-up/pull-down resistors
* Synchronous and asynchronous interrupt handlers
* Software-based PWM implementation
* Optional `embedded-hal` trait implementations (`digital::{InputPin, OutputPin, StatefulOutputPin, ToggleableOutputPin}`, `Pwm`, `PwmPin`)

### [I2C](https://docs.golemparts.com/rppal/latest/i2c)

The Broadcom Serial Controller (BSC) peripheral controls a proprietary bus compliant with the I2C bus/interface. RPPAL communicates with the BSC using the `i2cdev` character device.

#### Features

* Single master, 7-bit slave addresses, transfer rates up to 400 kbit/s (Fast-mode)
* I2C basic read/write, block read/write, combined write+read
* SMBus protocols: Quick Command, Send/Receive Byte, Read/Write Byte/Word, Process Call, Block Write, PEC
* Optional `embedded-hal` trait implementations (`blocking::i2c::{Read, Write, WriteRead}`)

### [PWM](https://docs.golemparts.com/rppal/latest/pwm)

RPPAL controls the Raspberry Pi's PWM peripheral through the `pwm` sysfs interface.

#### Features

* Up to two hardware PWM channels
* Configurable frequency, duty cycle and polarity
* Optional `embedded-hal` trait implementations (`Pwm`, `PwmPin`)

### [SPI](https://docs.golemparts.com/rppal/latest/spi)

RPPAL controls the Raspberry Pi's main and auxiliary SPI peripherals through the `spidev` character device.

#### Features

* SPI master, mode 0-3, Slave Select active-low/active-high, 8 bits per word, configurable clock speed
* Half-duplex reads, writes, and multi-segment transfers
* Full-duplex transfers and multi-segment transfers
* Customizable options for each segment in a multi-segment transfer (clock speed, delay, SS change)
* Reverse bit order helper function
* Optional `embedded-hal` trait implementations (`blocking::spi::{Transfer, Write}`, `spi::FullDuplex`)

### [UART](https://docs.golemparts.com/rppal/latest/uart)

RPPAL controls the Raspberry Pi's UART peripherals through the `ttyAMA0` (PL011) and `ttyS0` (mini UART) character devices. USB to serial adapters are controlled using the `ttyUSBx` and `ttyACMx` character devices.

#### Features

* Support for UART peripherals (PL011, mini UART) and USB to serial adapters
* None/Even/Odd/Mark/Space parity, 5-8 data bits, 1-2 stop bits
* Transfer rates up to 4 Mbit/s (device-dependent)
* XON/XOFF software flow control
* RTS/CTS hardware flow control with automatic pin configuration
* Optional `embedded-hal` trait implementations (`blocking::serial::Write`, `serial::{Read, Write}`)

## Cross compilation

If you're not working directly on a Raspberry Pi, you'll have to cross-compile your code for the appropriate ARM architecture. Check out [this guide](https://github.com/japaric/rust-cross) for more information, or try the [cross](https://github.com/japaric/cross) project for "zero setup" cross compilation.

### Cargo

While additional steps may be necessary to cross-compile binaries on your platform, checking your code with `cargo check` only requires the installation of an appropriate target. Most Raspberry Pi models need the `armv7-unknown-linux-gnueabihf` target. For some models, like the Raspberry Pi Zero, a different target triple is required.

Install the relevant target using `rustup`.

```bash
rustup target install armv7-unknown-linux-gnueabihf
```

In the root directory of your project, create a `.cargo` subdirectory, and then save the following snippet to `.cargo/config`.

```toml
[build]
target = "armv7-unknown-linux-gnueabihf"
```

### RLS

RLS needs to be made aware of the target platform by setting the `rust.target` configuration option. The location of this option is IDE-specific.

#### Visual Studio Code

In the root directory of your project, create a `.vscode` subdirectory, and then save the following snippet to `.vscode/settings.json`.

```json
{
    "rust.target": "armv7-unknown-linux-gnueabihf"
}
```

## Caution

Always be careful when working with the Raspberry Pi's peripherals, especially if you attach any external components to the GPIO pins. Improper use can lead to permanent damage.

## Copyright and license

Copyright (c) 2017-2019 Rene van der Meer. Released under the [MIT license](LICENSE).

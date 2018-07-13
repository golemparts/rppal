# RPPAL - Raspberry Pi Peripheral Access Library

[![Build Status](https://travis-ci.org/golemparts/rppal.svg?branch=master)](https://travis-ci.org/golemparts/rppal)
[![crates.io](https://meritbadge.herokuapp.com/rppal)](https://crates.io/crates/rppal)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

RPPAL is a Rust library that provides access to the Raspberry Pi's GPIO, I2C, PWM and SPI peripherals. Support for [additional peripherals](https://github.com/golemparts/rppal/projects/1) will be added in future updates. The library is compatible with the Raspberry Pi A, A+, B, B+, 2B, 3B, 3B+, Compute, Compute 3, Zero and Zero W.

Backwards compatibility for minor revisions isn't guaranteed until the library reaches v1.0.0.

## Documentation

Documentation for the latest release can be found at [docs.golemparts.com/rppal](https://docs.golemparts.com/rppal). Documentation for earlier releases is stored at [docs.rs/rppal](https://docs.rs/rppal).

## Supported peripherals

### GPIO

To ensure fast performance, RPPAL interfaces with the GPIO peripheral by directly accessing the registers through either `/dev/gpiomem` or `/dev/mem`. GPIO interrupts are controlled using the `/dev/gpiochipN` character device.

#### Features

* Get/set pin modes
* Read/write pin logic levels
* Activate built-in pull-up/pull-down resistors
* Configure synchronous and asynchronous interrupt handlers

### I2C

The Broadcom Serial Controller (BSC) peripheral controls a proprietary bus compliant with the I2C bus/interface. RPPAL communicates with the BSC using the i2cdev device interface.

#### Features

* Single master, 7-bit slave addresses, transfer rates up to 400kbit/s (Fast-mode)
* I2C basic read/write, block read/write, combined write+read
* SMBus protocols: Quick Command, Send/Receive Byte, Read/Write Byte/Word, Process Call, Block Write, PEC

### PWM

_Coming in Release 0.8.0._

RPPAL configures the Raspberry Pi's PWM peripheral through the `/sys/class/pwm` sysfs interface.

#### Features

* Up to two hardware PWM channels
* Configurable frequency/period, duty cycle and polarity

### SPI

RPPAL accesses the Raspberry Pi's main and auxiliary SPI peripherals through the spidev device interface.

#### Features

* SPI master, mode 0-3, Slave Select active-low/active-high, 8 bits per word, configurable clock speed
* Half-duplex reads, writes, and multi-segment transfers
* Full-duplex transfers and multi-segment transfers
* Customizable options for each segment in a multi-segment transfer (clock speed, delay, SS change)
* Reverse bit order helper function

### UART

_Coming in Release 0.8.0._

RPPAL controls the Raspberry Pi's main and auxiliary UART peripherals through the ttyAMA0 and ttyS0 device interfaces.
In addition to the built-in UARTs, communicating with USB serial devices is supported through ttyUSBx and ttyACMx.

#### Features

* UART0 (PL011): None/even/odd/mark/space parity, 5-8 data bits, 1-2 stop bits, stable transfer rates up to ~1Mbit/s, RTS/CTS hardware flow control
* UART1 (Mini UART): No parity bit, 7-8 data bits, 1 stop bit, limited transfer rates tied to the VPU core frequency, limited flow control
* USB/ACM: Transfer rates up to 4Mbit/s

## Usage

Add a dependency for `rppal` to your `Cargo.toml`.

```toml
[dependencies]
rppal = "0.7"
```

Link and import `rppal` from your crate root.

```rust
extern crate rppal;
```

Call `Gpio::new()` to create a new Gpio instance with the default settings. In production code, you'll want to parse the result rather than unwrap it.

```rust
use rppal::gpio::Gpio;

let mut gpio = Gpio::new().unwrap();
```

## Example

```rust
extern crate rppal;

use std::thread;
use std::time::Duration;

use rppal::gpio::{Gpio, Mode, Level};
use rppal::system::DeviceInfo;

// The GPIO module uses BCM pin numbering. BCM GPIO 18 is tied to physical pin 12.
const GPIO_LED: u8 = 18;

fn main() {
    let device_info = DeviceInfo::new().unwrap();
    println!("Model: {} (SoC: {})", device_info.model(), device_info.soc());

    let mut gpio = Gpio::new().unwrap();
    gpio.set_mode(GPIO_LED, Mode::Output);

    // Blink an LED attached to the pin on and off
    gpio.write(GPIO_LED, Level::High);
    thread::sleep(Duration::from_millis(500));
    gpio.write(GPIO_LED, Level::Low);
}
```

## Caution

Always be careful when working with the Raspberry Pi's peripherals, especially if you attach any external components to the GPIO pins. Improper use can lead to permanent damage.

## Cross compilation

If you're not working directly on a Raspberry Pi, you'll likely need to cross compile your code for the appropriate ARM architecture. Check out [this guide](https://github.com/japaric/rust-cross) for more information, or try the [cross](https://github.com/japaric/cross) project for "zero setup" cross compilation.

## Copyright and license

Copyright (c) 2017-2018 Rene van der Meer. Released under the [MIT license](LICENSE).

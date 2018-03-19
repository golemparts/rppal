# RPPAL - Raspberry Pi Peripheral Access Library

[![Build Status](https://travis-ci.org/golemparts/rppal.svg?branch=master)](https://travis-ci.org/golemparts/rppal)
[![crates.io](https://meritbadge.herokuapp.com/rppal)](https://crates.io/crates/rppal)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

RPPAL is a Rust library that provides access to the Raspberry Pi GPIO peripheral through either `/dev/gpiomem` or `/dev/mem`. Support for additional peripherals, as well as useful helper functions, will be added in future updates. The library is compatible with the Raspberry Pi A, A+, B, B+, 2B, 3B, 3B+, Compute, Compute 3, Zero and Zero W.

Backwards compatibility for minor revisions isn't guaranteed until the library reaches v1.0.0.

## Documentation

Documentation for the latest release can be found at [docs.golemparts.com/rppal](https://docs.golemparts.com/rppal). Documentation for earlier releases is stored at [docs.rs/rppal](https://docs.rs/rppal).

## Usage

Add a dependency for `rppal` to your `Cargo.toml`.

```toml
[dependencies]
rppal = "0.4"
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

// The GPIO module uses BCM pin numbering. BCM 18 equates to physical pin 12.
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

Always be careful when working with the Raspberry Pi's GPIO. Improper use can lead to permanently damaging the Pi and/or external components.

## Cross compilation

If you're not working directly on a Raspberry Pi, you'll likely need to cross compile your code for the appropriate ARM architecture. Check out [this guide](https://github.com/japaric/rust-cross) for more information, or try the [cross](https://github.com/japaric/cross) project for "zero setup" cross compilation.

## Copyright and license

Copyright (c) 2017-2018 Rene van der Meer. Released under the [MIT license](LICENSE).

# RPPAL - Raspberry Pi Peripheral Access Library

[![Build Status](https://travis-ci.org/golemparts/rppal.svg?branch=master)](https://travis-ci.org/golemparts/rppal)
[![crates.io](https://meritbadge.herokuapp.com/rppal)](https://crates.io/crates/rppal)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

RPPAL is a Rust library that provides access to the Raspberry Pi's GPIO, I2C and SPI peripherals. Support for [additional peripherals](https://github.com/golemparts/rppal/projects/1) will be added in future updates. The library is compatible with the Raspberry Pi A, A+, B, B+, 2B, 3B, 3B+, Compute, Compute 3, Zero and Zero W.

Backwards compatibility for minor revisions isn't guaranteed until the library reaches v1.0.0.

## Documentation

Documentation for the latest release can be found at [docs.golemparts.com/rppal](https://docs.golemparts.com/rppal). Documentation for earlier releases is stored at [docs.rs/rppal](https://docs.rs/rppal).

## Supported peripherals

### GPIO

To ensure fast performance, RPPAL interfaces with the GPIO peripheral by directly accessing the registers through either `/dev/gpiomem` or `/dev/mem`. GPIO interrupts are controlled using the sysfs interface.

#### Features

* Get/set pin modes
* Read/write pin logic levels
* Activate built-in pull-up/pull-down resistors
* Configure synchronous and asynchronous interrupt handlers

### I2C

The Broadcom Serial Controller (BSC) peripheral controls a proprietary bus compliant with the I2C bus/interface. RPPAL communicates with the BSC using the i2cdev device interface.

#### Features

* Single master, 7-bit slave addresses, transfer rates up to 400kbit/s (Fast-mode)
* I2C basic read/write, block read/write
* SMBus protocols: Quick Command, Send/Receive Byte, Read/Write Byte/Word, Process Call, Block Write, PEC

#### Unsupported features

Some I2C and SMBus features aren't fully supported by the i2cdev interface, the underlying driver or
the BCM283x SoC: 10-bit slave addresses, SMBus Block Read, SMBus Block Process Call, SMBus Host Notify,
SMBus Read/Write 32/64, and the SMBus Address Resolution Protocol.

While clock stretching is supported, a bug exists in the implementation on the BCM283x SoC that will result
in corrupted data when a slave device tries to use clock stretching at arbitrary points during the transfer.
Clock stretching only works properly during read operations, directly after the ACK phase, when the additional
delay is longer than half of a clock period. More information can be found [here](https://elinux.org/BCM2835_datasheet_errata#p35_I2C_clock_stretching).

A possible workaround for slave devices that require clock stretching at other points during the transfer is
to use a bit-banged software I2C bus by configuring the `i2c-gpio` device tree overlay as described in `/boot/overlays/README`.

### SPI

RPPAL accesses the Raspberry Pi's main and auxiliary SPI peripherals through the spidev device interface.

#### Features

* SPI master, mode 0-3, Slave Select active-low/active-high, 8 bits per word, configurable clock speed
* Half-duplex reads, writes, and multi-segment transfers
* Full-duplex transfers and multi-segment transfers
* Customizable options for each segment in a multi-segment transfer (clock speed, delay, SS change)
* Reverse bit order helper function

#### Unsupported features

Some features exposed by the generic spidev interface aren't fully
supported by the underlying driver or the BCM283x SoC: SPI_LSB_FIRST (LSB
first bit order), SPI_3WIRE (bidirectional mode), SPI_LOOP (loopback mode),
SPI_NO_CS (no Slave Select), SPI_READY (slave ready signal),
SPI_TX_DUAL/SPI_RX_DUAL (dual SPI), SPI_TX_QUAD/SPI_RX_QUAD (quad SPI),
and any number of bits per word other than 8.

If your slave device requires SPI_LSB_FIRST, you can use the
`reverse_bits` function instead to reverse the bit order in software.

SPI_LOOP mode can be achieved by connecting the MOSI and MISO pins
together.

SPI_NO_CS can be implemented by connecting the Slave Select pin on your
slave device to any other available GPIO pin on the Pi, and manually
changing it to high and low as needed.

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

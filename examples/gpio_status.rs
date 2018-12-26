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

// gpio_status.rs
//
// Retrieves the mode and logic level for each of the pins on the standard
// 40-pin GPIO header, and displays the results in an ASCII table.

use std::fmt;
use std::process::exit;

use rppal::gpio::Gpio;
use rppal::system::{DeviceInfo, Model};

enum PinType {
    Gpio(u8),
    Ground,
    Power3v3,
    Power5v,
}

impl fmt::Display for PinType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            PinType::Gpio(pin) => write!(f, "GPIO{}", pin),
            PinType::Ground => write!(f, "{:<5}", "GND"),
            PinType::Power3v3 => write!(f, "{:<5}", "3.3 V"),
            PinType::Power5v => write!(f, "{:<5}", "5 V"),
        }
    }
}

const HEADER_40: [PinType; 40] = [
    PinType::Power3v3, // Physical pin 1
    PinType::Power5v,  // Physical pin 2
    PinType::Gpio(2),  // Physical pin 3
    PinType::Power5v,  // Physical pin 4
    PinType::Gpio(3),  // Physical pin 5
    PinType::Ground,   // Physical pin 6
    PinType::Gpio(4),  // Physical pin 7
    PinType::Gpio(14), // Physical pin 8
    PinType::Ground,   // Physical pin 9
    PinType::Gpio(15), // Physical pin 10
    PinType::Gpio(17), // Physical pin 11
    PinType::Gpio(18), // Physical pin 12
    PinType::Gpio(27), // Physical pin 13
    PinType::Ground,   // Physical pin 14
    PinType::Gpio(22), // Physical pin 15
    PinType::Gpio(23), // Physical pin 16
    PinType::Power3v3, // Physical pin 17
    PinType::Gpio(24), // Physical pin 18
    PinType::Gpio(10), // Physical pin 19
    PinType::Ground,   // Physical pin 20
    PinType::Gpio(9),  // Physical pin 21
    PinType::Gpio(25), // Physical pin 22
    PinType::Gpio(11), // Physical pin 23
    PinType::Gpio(8),  // Physical pin 24
    PinType::Ground,   // Physical pin 25
    PinType::Gpio(7),  // Physical pin 26
    PinType::Gpio(0),  // Physical pin 27
    PinType::Gpio(1),  // Physical pin 28
    PinType::Gpio(5),  // Physical pin 29
    PinType::Ground,   // Physical pin 30
    PinType::Gpio(6),  // Physical pin 31
    PinType::Gpio(12), // Physical pin 32
    PinType::Gpio(13), // Physical pin 33
    PinType::Ground,   // Physical pin 34
    PinType::Gpio(19), // Physical pin 35
    PinType::Gpio(16), // Physical pin 36
    PinType::Gpio(26), // Physical pin 37
    PinType::Gpio(20), // Physical pin 38
    PinType::Ground,   // Physical pin 39
    PinType::Gpio(21), // Physical pin 40
];

fn print_pin(
    pin: usize,
    gpio: impl fmt::Display,
    mode: impl fmt::Display,
    level: impl fmt::Display,
) {
    if pin % 2 != 0 {
        print!("| {:>4} | {:<5} | {:>1} | {:>2} |", gpio, mode, level, pin);
    } else {
        println!(" {:>2} | {:>1} | {:<5} | {:>4} |", pin, level, mode, gpio);
    }
}

fn print_gpio_status() {
    let gpio = Gpio::new().unwrap_or_else(|e| {
        eprintln!("Error: Can't access GPIO peripheral ({})", e);
        exit(1);
    });

    println!("+------+-------+---+---------+---+-------+------+");
    println!("| GPIO | Mode  | L |   Pin   | L | Mode  | GPIO |");
    println!("+------+-------+---+----+----+---+-------+------+");

    for (idx, pin_type) in HEADER_40.iter().enumerate() {
        match pin_type {
            PinType::Gpio(bcm_gpio) => {
                let pin = gpio.get(*bcm_gpio).unwrap_or_else(|| {
                    eprintln!("Error: Can't access GPIO pin {}", *bcm_gpio);
                    exit(1);
                });

                print_pin(
                    idx + 1,
                    bcm_gpio,
                    format!("{}", pin.mode()).to_uppercase(),
                    pin.read() as u8,
                );
            }
            _ => print_pin(idx + 1, "", pin_type, ""),
        };
    }

    println!("+------+-------+---+----+----+---+-------+------+");
}

fn main() {
    match DeviceInfo::new()
        .unwrap_or_else(|e| {
            eprintln!("Error: Can't identify Raspberry Pi model ({})", e);
            exit(1);
        })
        .model()
    {
        Model::RaspberryPiAPlus
        | Model::RaspberryPiBPlus
        | Model::RaspberryPi2B
        | Model::RaspberryPi3APlus
        | Model::RaspberryPi3B
        | Model::RaspberryPi3BPlus
        | Model::RaspberryPiZero
        | Model::RaspberryPiZeroW => print_gpio_status(),
        model => {
            eprintln!("Error: No GPIO header information available for {}", model);
            exit(1);
        }
    }
}

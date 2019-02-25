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

// gpio_servo_softpwm.rs - Rotates a servo connected to a GPIO pin using
// software-based PWM.
//
// Calibrate your servo beforehand, and change the values listed below to fall
// within your servo's safe limits to prevent potential damage. Don't power the
// servo directly from the Pi's GPIO header. Current spikes during power-up and
// stalls could otherwise damage your Pi, or cause your Pi to spontaneously
// reboot, corrupting your microSD card. If you're powering the servo using a
// separate power supply, remember to connect the grounds of the Pi and the
// power supply together.
//
// Software-based PWM is inherently inaccurate on a multi-threaded OS due to
// scheduling/preemption. If an accurate or faster PWM signal is required, use
// the hardware PWM peripheral instead. Check out the pwm_servo.rs example to
// learn how to control a servo using hardware PWM.

use std::error::Error;
use std::thread;
use std::time::Duration;

use rppal::gpio::Gpio;

// Gpio uses BCM pin numbering. BCM GPIO 23 is tied to physical pin 16.
const GPIO_PWM: u8 = 23;

// Servo configuration. Change these values based on your servo's verified safe
// minimum and maximum values.
//
// Period: 20 ms (50 Hz). Pulse width: min. 1200 µs, neutral 1500 µs, max. 1800 µs.
const PERIOD_MS: u64 = 20;
const PULSE_MIN_US: u64 = 1200;
const PULSE_NEUTRAL_US: u64 = 1500;
const PULSE_MAX_US: u64 = 1800;

fn main() -> Result<(), Box<dyn Error>> {
    // Retrieve the GPIO pin and configure it as an output.
    let mut pin = Gpio::new()?.get(GPIO_PWM)?.into_output();

    // Enable software-based PWM with the specified period, and rotate the servo by
    // setting the pulse width to its maximum value.
    pin.set_pwm(
        Duration::from_millis(PERIOD_MS),
        Duration::from_micros(PULSE_MAX_US),
    )?;

    // Sleep for 500 ms while the servo moves into position.
    thread::sleep(Duration::from_millis(500));

    // Rotate the servo to the opposite side.
    pin.set_pwm(
        Duration::from_millis(PERIOD_MS),
        Duration::from_micros(PULSE_MIN_US),
    )?;

    thread::sleep(Duration::from_millis(500));

    // Rotate the servo to its neutral (center) position in small steps.
    for pulse in (PULSE_MIN_US..=PULSE_NEUTRAL_US).step_by(10) {
        pin.set_pwm(
            Duration::from_millis(PERIOD_MS),
            Duration::from_micros(pulse),
        )?;
        thread::sleep(Duration::from_millis(20));
    }

    Ok(())

    // When the pin variable goes out of scope, software-based PWM is automatically disabled.
    // You can manually disable PWM by calling the clear_pwm() method.
}

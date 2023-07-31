// pwm_servo.rs - Rotates a servo using hardware PWM.
//
// Calibrate your servo beforehand, and change the values listed below to fall
// within your servo's safe limits to prevent potential damage. Don't power the
// servo directly from the Pi's GPIO header. Current spikes during power-up and
// stalls could otherwise damage your Pi, or cause your Pi to spontaneously
// reboot, corrupting your microSD card. If you're powering the servo using a
// separate power supply, remember to connect the grounds of the Pi and the
// power supply together.
//
// Interrupting the process by pressing Ctrl-C causes the application to exit
// immediately without disabling the PWM channel. Check out the
// gpio_blinkled_signals.rs example to learn how to properly handle incoming
// signals to prevent an abnormal termination.

use std::error::Error;
use std::thread;
use std::time::Duration;

use rppal::pwm::{Channel, Polarity, Pwm};

// Servo configuration. Change these values based on your servo's verified safe
// minimum and maximum values.
//
// Period: 20 ms (50 Hz). Pulse width: min. 1200 µs, neutral 1500 µs, max. 1800 µs.
const PERIOD_MS: u64 = 20;
const PULSE_MIN_US: u64 = 1200;
const PULSE_NEUTRAL_US: u64 = 1500;
const PULSE_MAX_US: u64 = 1800;

fn main() -> Result<(), Box<dyn Error>> {
    // Enable PWM channel 0 (BCM GPIO 18, physical pin 12) with the specified period,
    // and rotate the servo by setting the pulse width to its maximum value.
    let pwm = Pwm::with_period(
        Channel::Pwm0,
        Duration::from_millis(PERIOD_MS),
        Duration::from_micros(PULSE_MAX_US),
        Polarity::Normal,
        true,
    )?;

    // Sleep for 500 ms while the servo moves into position.
    thread::sleep(Duration::from_millis(500));

    // Rotate the servo to the opposite side.
    pwm.set_pulse_width(Duration::from_micros(PULSE_MIN_US))?;

    thread::sleep(Duration::from_millis(500));

    // Rotate the servo to its neutral (center) position in small steps.
    for pulse in (PULSE_MIN_US..=PULSE_NEUTRAL_US).step_by(10) {
        pwm.set_pulse_width(Duration::from_micros(pulse))?;
        thread::sleep(Duration::from_millis(20));
    }

    Ok(())

    // When the pwm variable goes out of scope, the PWM channel is automatically disabled.
    // You can manually disable the channel by calling the Pwm::disable() method.
}

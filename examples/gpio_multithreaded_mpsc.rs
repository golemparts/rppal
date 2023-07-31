// gpio_multithreaded_mpsc.rs - Blinks an LED on a separate thread using an
// MPSC channel.
//
// Remember to add a resistor of an appropriate value in series, to prevent
// exceeding the maximum current rating of the GPIO pin and the LED.

use std::error::Error;
use std::sync::mpsc::channel;
use std::thread;
use std::time::Duration;

use rppal::gpio::Gpio;

// Gpio uses BCM pin numbering. BCM GPIO 23 is tied to physical pin 16.
const GPIO_LED: u8 = 23;

fn main() -> Result<(), Box<dyn Error>> {
    // Construct an asynchronous channel. Sender can be cloned if it needs to be shared with other threads.
    let (sender, receiver) = channel();

    let led_thread = thread::spawn(move || -> Result<(), rppal::gpio::Error> {
        // Retrieve the GPIO pin and configure it as an output.
        let mut pin = Gpio::new()?.get(GPIO_LED)?.into_output_low();

        // Wait for an incoming message. Loop until a None is received.
        while let Some(count) = receiver.recv().unwrap() {
            println!("Blinking the LED {} times.", count);
            for _ in 0u8..count {
                pin.set_high();
                thread::sleep(Duration::from_millis(250));
                pin.set_low();
                thread::sleep(Duration::from_millis(250));
            }
        }

        Ok(())
    });

    // Request 3 blinks. We're using an asynchronous channel, so send() returns immediately.
    sender.send(Some(3))?;

    // Sending None terminates the while loop on the LED thread.
    sender.send(None)?;

    // Wait until the LED thread has finished executing.
    led_thread.join().unwrap()?;

    Ok(())
}

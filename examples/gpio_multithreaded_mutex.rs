// gpio_multithreaded_mutex.rs - Blinks an LED from multiple threads.
//
// Remember to add a resistor of an appropriate value in series, to prevent
// exceeding the maximum current rating of the GPIO pin and the LED.

use std::error::Error;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use rppal::gpio::Gpio;

// Gpio uses BCM pin numbering. BCM GPIO 23 is tied to physical pin 16.
const GPIO_LED: u8 = 23;
const NUM_THREADS: usize = 3;

fn main() -> Result<(), Box<dyn Error>> {
    // Retrieve the GPIO pin and configure it as an output.
    let output_pin = Arc::new(Mutex::new(Gpio::new()?.get(GPIO_LED)?.into_output_low()));

    // Populate a Vec with threads so we can call join() on them later.
    let mut threads = Vec::with_capacity(NUM_THREADS);
    (0..NUM_THREADS).for_each(|thread_id| {
        // Clone the Arc so it can be moved to the spawned thread.
        let output_pin_clone = Arc::clone(&output_pin);

        threads.push(thread::spawn(move || {
            // Lock the Mutex on the spawned thread to get exclusive access to the OutputPin.
            let mut pin = output_pin_clone.lock().unwrap();
            println!("Blinking the LED from thread {}.", thread_id);
            pin.set_high();
            thread::sleep(Duration::from_millis(250));
            pin.set_low();
            thread::sleep(Duration::from_millis(250));
            // The MutexGuard is automatically dropped here.
        }));
    });

    // Lock the Mutex on the main thread to get exclusive access to the OutputPin.
    let mut pin = output_pin.lock().unwrap();
    println!("Blinking the LED from the main thread.");
    pin.set_high();
    thread::sleep(Duration::from_millis(250));
    pin.set_low();
    thread::sleep(Duration::from_millis(250));
    // Manually drop the MutexGuard so the Mutex doesn't stay locked indefinitely.
    drop(pin);

    // Wait until all threads have finished executing.
    threads
        .into_iter()
        .for_each(|thread| thread.join().unwrap());

    Ok(())
}

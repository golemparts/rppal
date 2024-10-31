// gpio_shared_button_state.rs - Stops the program until a certain amount of event input changes via
// a non-global shared variable (that can be done using OnceCell for example), this requires a Mutex
// as it goes across threads and Arc to make sure we have the same entry everywhere.

use std::error::Error;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use rppal::gpio::{Gpio, Event, Trigger};

const INPUT_PIN_GPIO: u8 = 27;
const STOP_AFTER_N_CHANGES: u8 = 5;

// The function we will run upon a Trigger
fn input_callback(event: Event, my_data: Arc<Mutex<u8>>) {
    println!("Event: {:?}", event);
    *my_data.lock().unwrap() += 1;
}

fn main() -> Result<(), Box<dyn Error>> {
    // Initialize our data, in this case it's just a number.
    let shared_state = Arc::new(Mutex::new(0));

    // Configure the input pin.
    let mut input_pin = Gpio::new()?.get(INPUT_PIN_GPIO)?.into_input_pulldown();

    // We need to clone this as set_async_interrupt will move it and cant be used afterward if so
    let shared_state_hold = shared_state.clone();
    input_pin.set_async_interrupt(
        Trigger::FallingEdge,
        Some(Duration::from_millis(50)),
        move |event| {
            // Note: you can also add more parameters here!
            input_callback(event, shared_state_hold.clone());
        },
    )?;

    // We check constantly if we have reached our number of changes.
    loop {
        if *shared_state.lock().unwrap() >= STOP_AFTER_N_CHANGES {
            // Reached it, exiting the program.
            println!("Reached {STOP_AFTER_N_CHANGES} events, exiting...");
            break;
        }

        // Suppose we do some work here that takes a second, the shorter the work takes, the quicker
        // we will quit upon reaching our condition.
        println!("Still waiting...");
        std::thread::sleep(Duration::from_secs(1));
    }

    Ok(())
}

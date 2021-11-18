// uart_blocking_read.rs - Blocks while waiting for incoming serial data.

use std::error::Error;
use std::time::Duration;

use rppal::uart::{Parity, Uart};

fn main() -> Result<(), Box<dyn Error>> {
    // Connect to the primary UART and configure it for 115.2 kbit/s, no
    // parity bit, 8 data bits and 1 stop bit.
    let mut uart = Uart::new(115_200, Parity::None, 8, 1)?;

    // Configure read() to block until at least 1 byte is received.
    uart.set_read_mode(1, Duration::default())?;

    let mut buffer = [0u8; 1];
    loop {
        // Fill the buffer variable with any incoming data.
        if uart.read(&mut buffer)? > 0 {
            println!("Received byte: {}", buffer[0]);
        }
    }
}

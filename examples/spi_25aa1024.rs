// spi_25aa1024.rs - Transfers data to a Microchip 25AA1024 serial EEPROM using SPI.

use std::error::Error;

use rppal::spi::{Bus, Mode, Segment, SlaveSelect, Spi};

// Instruction set.
const WRITE: u8 = 0b0010; // Write data, starting at the selected address.
const READ: u8 = 0b0011; // Read data, starting at the selected address.
const RDSR: u8 = 0b0101; // Read the STATUS register.
const WREN: u8 = 0b0110; // Set the write enable latch (enable write operations).

const WIP: u8 = 1; // Write-In-Process bit mask for the STATUS register.

fn main() -> Result<(), Box<dyn Error>> {
    // Configure the SPI peripheral. The 24AA1024 clocks in data on the first
    // rising edge of the clock signal (SPI mode 0). At 3.3 V, clock speeds of up
    // to 10 MHz are supported.
    let mut spi = Spi::new(Bus::Spi0, SlaveSelect::Ss0, 8_000_000, Mode::Mode0)?;

    // Set the write enable latch using the WREN instruction. This is required
    // before any data can be written. The write enable latch is automatically
    // reset after a WRITE instruction is successfully executed.
    spi.write(&[WREN])?;

    // Use the WRITE instruction to select memory address 0 and write 5 bytes
    // (1, 2, 3, 4, 5). Addresses are specified as 24-bit values, but the 7 most
    // significant bits are ignored.
    spi.write(&[WRITE, 0, 0, 0, 1, 2, 3, 4, 5])?;

    // Read the STATUS register by writing the RDSR instruction, and then reading
    // a single byte. Loop until the WIP bit is set to 0, indicating the write
    // operation is completed. transfer_segments() will keep the Slave Select line
    // active until both segments have been transferred.
    let mut buffer = [0u8; 1];
    loop {
        spi.transfer_segments(&[
            Segment::with_write(&[RDSR]),
            Segment::with_read(&mut buffer),
        ])?;

        if buffer[0] & WIP == 0 {
            break;
        }
    }

    // Use the READ instruction to select memory address 0, specified as a 24-bit
    // value, and then read 5 bytes.
    let mut buffer = [0u8; 5];
    spi.transfer_segments(&[
        Segment::with_write(&[READ, 0, 0, 0]),
        Segment::with_read(&mut buffer),
    ])?;

    println!("Bytes read: {:?}", buffer);

    Ok(())
}

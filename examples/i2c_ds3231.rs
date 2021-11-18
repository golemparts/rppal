// i2c_ds3231.rs - Sets and retrieves the time on a Maxim Integrated DS3231
// RTC using I2C.

use std::error::Error;
use std::thread;
use std::time::Duration;

use rppal::i2c::I2c;

// DS3231 I2C default slave address.
const ADDR_DS3231: u16 = 0x68;

// DS3231 register addresses.
const REG_SECONDS: usize = 0x00;
const REG_MINUTES: usize = 0x01;
const REG_HOURS: usize = 0x02;

// Helper functions to encode and decode binary-coded decimal (BCD) values.
fn bcd2dec(bcd: u8) -> u8 {
    (((bcd & 0xF0) >> 4) * 10) + (bcd & 0x0F)
}

fn dec2bcd(dec: u8) -> u8 {
    ((dec / 10) << 4) | (dec % 10)
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut i2c = I2c::new()?;

    // Set the I2C slave address to the device we're communicating with.
    i2c.set_slave_address(ADDR_DS3231)?;

    // Set the time to 11:59:50 AM. Start at register address 0x00 (Seconds) and
    // write 3 bytes, overwriting the Seconds, Minutes and Hours registers.
    // Setting bit 6 of the Hours register indicates we're using a 12-hour
    // format. Leaving bit 5 unset indicates AM.
    i2c.block_write(
        REG_SECONDS as u8,
        &[dec2bcd(50), dec2bcd(59), dec2bcd(11) | (1 << 6)],
    )?;

    let mut reg = [0u8; 3];
    loop {
        // Start at register address 0x00 (Seconds) and read the values of the
        // next 3 registers (Seconds, Minutes, Hours) into the reg variable.
        i2c.block_read(REG_SECONDS as u8, &mut reg)?;

        // Display the retrieved time in the appropriate format based on bit 6 of
        // the Hours register.
        if reg[REG_HOURS] & (1 << 6) > 0 {
            // 12-hour format.
            println!(
                "{:0>2}:{:0>2}:{:0>2} {}",
                bcd2dec(reg[REG_HOURS] & 0x1F),
                bcd2dec(reg[REG_MINUTES]),
                bcd2dec(reg[REG_SECONDS]),
                if reg[REG_HOURS] & (1 << 5) > 0 {
                    "PM"
                } else {
                    "AM"
                }
            );
        } else {
            // 24-hour format.
            println!(
                "{:0>2}:{:0>2}:{:0>2}",
                bcd2dec(reg[REG_HOURS] & 0x3F),
                bcd2dec(reg[REG_MINUTES]),
                bcd2dec(reg[REG_SECONDS])
            );
        }

        thread::sleep(Duration::from_secs(1));
    }
}

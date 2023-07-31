# Examples

Before running any examples that interface with external components, read through the comments in the source code and take the necessary steps and precautions, where applicable, to prevent potential damage to your Raspberry Pi or other hardware.

`gpio_blinkled.rs` - Blinks an LED in a loop.

`gpio_blinkled_signals.rs` - Blinks an LED in a loop, while handling any incoming `SIGINT` (<kbd>Ctrl</kbd> + <kbd>C</kbd>) and `SIGTERM` signals so the pin's state can be reset before the application exits.

`gpio_multithreaded_mpsc.rs` - Blinks an LED on a separate thread using an MPSC channel.

`gpio_multithreaded_mutex.rs` - Blinks an LED from multiple threads.

`gpio_servo_softpwm.rs` - Rotates a servo using software-based PWM.

`gpio_status.rs` - Retrieves the mode and logic level for each of the pins on the 26-pin or 40-pin GPIO header, and displays the results in an ASCII table.

`i2c_ds3231.rs` - Sets and retrieves the time on a Maxim Integrated DS3231 RTC using I2C.

`pwm_blinkled.rs` - Blinks an LED using hardware PWM.

`pwm_servo.rs` - Rotates a servo using hardware PWM.

`spi_25aa1024.rs` - Transfers data to a Microchip 25AA1024 serial EEPROM using SPI.

`uart_blocking_read.rs` - Blocks while waiting for incoming serial data.

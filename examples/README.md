# Examples

`gpio_blinked.rs` - Blinks an LED attached to a GPIO pin in a loop.

`gpio_blinkled_signals.rs` - Blinks an LED attached to a GPIO pin in a loop, while handling any incoming SIGINT (Ctrl-C) and SIGTERM signals so the pin's state can be reset before the application exits.

`gpio_status.rs` - Retrieves the mode and logic level for each of the pins on the 26-pin or 40-pin GPIO header, and displays the results in an ASCII table.
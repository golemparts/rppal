# Changelog

## 0.11.3 (June 24, 2019)

* **DeviceInfo**: Add device identification support for Raspberry Pi 4 B. (Support for the new peripherals will be added in 0.12.0)

## 0.11.2 (May 2, 2019)

* Add `hal-unproven` feature flag (disabled by default), which enables `unproven` `embedded-hal` trait implementations. Note that `embedded-hal`'s `unproven` traits don't follow semver rules. Patch releases may introduce breaking changes.
* **Gpio**: Implement `Sync` trait for `IoPin` and `OutputPin`.
* **Gpio**: Implement `unproven` `embedded-hal` trait `digital::InputPin` for `Pin`, `InputPin`, `OutputPin` and `IoPin`.
* **Gpio**: Implement `unproven` `embedded-hal` traits `digital::{StatefulOutputPin, ToggleableOutputPin}` and `Pwm` for `OutputPin` and `IoPin`.
* **Gpio**: Remove internal `MSG_WAITING` flag from software PWM implementation to resolve an issue found in the wild causing delays in message processing (contributed by @aehmlo).
* **Hal**: Add `hal` module, containing `embedded-hal` trait implementations that aren't tied to a specific peripheral.
* **Hal**: Implement `embedded-hal` traits `blocking::delay::{DelayMs, DelayUs}` for `Delay`.
* **Hal**: Implement `embedded-hal` trait `timer::CountDown` for `Timer` (contributed by @jacobrosenthal).
* **Pwm**: Implement `Display` trait for `Channel` and `Polarity`.
* **Pwm**: Implement `unproven` `embedded-hal` trait `Pwm` for `Pwm`.
* **Spi**: Implement `Display` trait for `BitOrder`, `Bus`, `Mode`, `Polarity` and `SlaveSelect`.
* **Spi**: Remove `From<Error>` implementation due to a conflict with `nb` v0.1.2 (contributed by @gferon).
* **Uart**: Add support for the PL011 and mini UART peripherals, USB to serial adapters, XON/XOFF software flow control and RTS/CTS hardware flow control.
* **Uart**: Implement `embedded-hal` traits `serial::{Read, Write}` and `blocking::serial::Write` for `Uart`.

## 0.11.1 (February 24, 2019)

* Fix incorrect data type conversion on 64-bit OSes when libc uses 64-bit `timespec` fields.

## 0.11.0 (February 20, 2019)

* Add `hal` feature flag (disabled by default), which includes `embedded-hal` trait implementations for all supported peripherals.
* Add `Gpio` example demonstrating software-based PWM.
* **DeviceInfo**: (Breaking change) Add support for Raspberry Pi Compute Module 3+.
* **DeviceInfo**: (Breaking change) Add hidden `Model::__Nonexhaustive` and `SoC::__Nonexhaustive` variants, indicating `Model` and `SoC` shouldn't be exhaustively matched. After this change, adding new variants to these enums when a new Raspberry Pi model is released won't be considered a breaking change anymore. This is a hack that can still be circumvented, but anyone that does so should be aware of the repercussions. This will be replaced once `#[non_exhaustive]` stabilizes.
* **Gpio**: Add software-based PWM to `OutputPin` and `IoPin` through `set_pwm()`, `set_pwm_frequency()` and `clear_pwm()`.
* **Gpio**: Add `is_set_low()` and `is_set_high()` to `OutputPin` to check the pin's output state.
* **Gpio**: Implement `embedded-hal` traits `digital::OutputPin` and `PwmPin` for `OutputPin` and `IoPin`.
* **I2c**: Implement `embedded-hal` traits `blocking::i2c::{Read, Write, WriteRead}` for `I2c`.
* **Pwm**: Add `reset_on_drop()` and `set_reset_on_drop()` to `Pwm` to optionally keep the PWM channel enabled on drop (contributed by @benkard).
* **Pwm**: Implement `embedded-hal` trait `PwmPin` for `Pwm`.
* **Spi**: Implement `embedded-hal` traits `spi::FullDuplex` and `blocking::spi::{Transfer, Write}` for `Spi`.

## 0.10.0 (January 18, 2019)

* (Breaking change) Transition to Rust 2018, requiring rustc v1.31.0 or newer to compile the library.
* Add new badge to `README.md`, indicating the required minimum rustc version.
* Add `Gpio`, `I2c`, `Pwm` and `Spi` examples to the examples subdirectory.
* Rewrite `Display` formatting for `Error`s in all modules to include more details when available.
* **DeviceInfo**: (Breaking change) Remove `DeviceInfo::peripheral_base()` and `DeviceInfo::gpio_offset()` from the public API.
* **Gpio**: (Breaking change) Move pin-specific methods from `Gpio` to the new `InputPin`/`OutputPin` structs. Access pins through `Gpio::get()` (contributed by @reitermarkus).
* **Gpio**: Add a new `IoPin` struct which allows mode switching between input, output or an alternate function.
* **Gpio**: `Gpio::get()` returns an owned unconfigured `Pin`, which can be used to read the pin's mode and logic level. Convert a `Pin` to an `InputPin`, `OutputPin` or `IoPin` through the various `Pin::into_` methods to access additional functionality.
* **Gpio**: Add a variety of convenience methods to `InputPin`, `OutputPin` and `IoPin` for common tasks.
* **Gpio**: (Breaking change) Remove `Error::NotInitialized`, `Error::UnknownMode` and `Error::InvalidPin` (contributed by @reitermarkus).
* **Gpio**: (Breaking change) Remove `Error::InstanceExists`. Multiple (thread-safe) `Gpio` instances can now exist simultaneously.
* **Gpio**: (Breaking change) Rename `Error::UnknownSoC` to `Error::UnknownModel` for consistency.
* **Gpio**: (Breaking change) Add relevant file path to `Error::PermissionDenied` to make it easier to solve file permission issues.
* **Gpio**: (Breaking change) Add `Error::PinNotAvailable`, returned by `Gpio::get()` to indicate a pin is already in use, or isn't available on the current Raspberry Pi model.
* **Gpio**: (Breaking change) Rename `clear_on_drop()`/`set_clear_on_drop()` to `reset_on_drop()`/`set_reset_on_drop()` for clarity.
* **Gpio**: (Breaking change) Change `Gpio::poll_interrupts()` `pins` input parameter and return type from `u8` to `&InputPin` (contributed by @reitermarkus).
* **Gpio**: When a pin goes out of scope, if an asynchronous interrupt trigger was configured for the pin, the polling thread will get stopped.
* **Gpio**: Disable built-in pull-up/pull-down resistors when a pin goes out of scope and `reset_on_drop` is set to true.
* **Gpio**: Implement `Clone` for `Gpio`.
* **I2c**: (Breaking change) Rename `Error::UnknownSoC` to `Error::UnknownModel` for consistency.
* **Pwm**: (Breaking change) Rename `duty_cycle()` to `pulse_width()` and `set_duty_cycle()` to `set_pulse_width()` to better reflect the specified value type.
* **Pwm**: (Breaking change) Rename `enabled()` to `is_enabled()` for more idiomatic predicates.
* **Pwm**: Add `duty_cycle()`, `set_duty_cycle()` and `frequency()` convenience methods that convert between frequency/duty cycle and period/pulse width.
* **Pwm**: Fix incorrect return values for `period()`, `duty_cycle()`, `polarity()` and `enabled()` caused by whitespace.
* **Spi**: (Breaking change) Rename `TransferSegment` to `Segment`.
* **Spi**: (Breaking change) `Segment::new()` parameters are no longer wrapped in `Option`. Use `Segment::with_read()` or `Segment::with_write()` instead when a full-duplex transfer isn't needed.
* **Spi**: Add `Segment::with_read()` and `Segment::with_write()` convenience methods for read operations without any outgoing data, or write operations where any incoming data should be discarded.

## 0.9.0 (November 15, 2018)

* **DeviceInfo**: (Breaking change) Add support for Raspberry Pi 3 A+.

## 0.8.1 (October 5, 2018)

* Add support for musl (contributed by @gferon).

## 0.8.0 (August 14, 2018)

* **Gpio**: Replace GPIO sysfs interface (`/sys/class/gpio`) for interrupts with GPIO character device (`/dev/gpiochipN`).
* **Pwm**: Add support for up to two hardware PWM channels with configurable period/frequency, duty cycle and polarity.
* **Spi**: Fix 0-length transfers caused by `TransferSegment` instances with `write_buffer` set to `None`.

## 0.7.1 (June 26, 2018)

* Revert the use of the recently stabilized `Duration::subsec_millis()` back to `Duration::subsec_nanos()` to allow older stable versions of the compiler to work.

## 0.7.0 (June 26, 2018)

* **DeviceInfo**: (Breaking change) Remove `Error::CantAccessProcCpuInfo`.
* **DeviceInfo**: Add additional options to automatically identify the Pi model when /proc/cpuinfo contains inaccurate data.
* **Gpio**: (Breaking change) Remove `Error::ChannelDisconnected`.
* **I2c**: Add support for I2C with basic read/write, block read/write, and write_read.
* **I2c**: Add support for SMBus with Quick Command, Send/Receive Byte, Read/Write Byte/Word, Process Call, Block Write, and PEC.
* Reduce external dependencies.

## 0.6.0 (June 1, 2018)

* **DeviceInfo**: (Breaking change) Return model and soc by value, rather than by reference.
* **DeviceInfo**: (Breaking change) Remove `SoC::Bcm2837` to reduce ambiguity. The Pi 3B and Compute Module 3 now return the more accurate `SoC::Bcm2837A1`.
* **DeviceInfo**: (Breaking change) Remove `SoC::Unknown`. An unknown SoC is now treated as a failure.
* **DeviceInfo**: Return the actual SoC based on the Raspberry Pi model, rather than the inaccurate `/proc/cpuinfo` data.
* **Gpio**: (Breaking change) Remove `InterruptError`. Merge remaining errors with `Error`.
* **Gpio**: (Breaking change) Replace all `DevGpioMem` and `DevMem` errors with `Error::PermissionDenied` and `Error::Io`.
* **Gpio**: (Breaking change) Change the return value for `poll_interrupt()` and `poll_interrupts()` to `Ok(Option)` on success, with `Some()` indicating an interrupt triggered, and `None` indicating a timeout occurred.
* **Gpio**: (Breaking change) Only a single instance of `Gpio` can exist at any time. Creating another instance before the existing one goes out of scope will return an `Error::InstanceExists`.
* **Spi**: Add support for SPI with half-duplex reads/writes and full-duplex multi-segment transfers.

## 0.5.1 (May 19, 2018)

* **Gpio**: Add `poll_interrupts()`, which waits for multiple synchronous interrupts at the same time.
* **Gpio**: Add public interface for `InterruptError`.
* Cleanup documentation.

## 0.5.0 (May 9, 2018)

* **DeviceInfo**: Add hardcoded Raspberry Pi 3 B+ SoC identifier, rather than relying on inaccurate info from `/proc/cpuinfo`.
* **Gpio**: Add support for asynchronous interrupts (`set_async_interrupt()`, `clear_async_interrupt()`).
* **Gpio**: Add support for synchronous interrupts (`set_interrupt()`, `clear_interrupt()`, `poll_interrupt()`).

## 0.4.0 (March 19, 2018)

* **Gpio**: Replace `&mut self` with `&self` where possible.

## 0.3.0 (March 16, 2018)

* **DeviceInfo**: (Breaking change) Add support for Raspberry Pi 3 B+.
* **DeviceInfo**: Set memory offsets based on model info rather than SoC.

## 0.2.0 (October 6, 2017)

* (Breaking change) To adhere to Rust's naming conventions, several structs and enums that had GPIO, IO, BCM or CPU in their name have been changed to `Gpio`, `Io`, `Bcm` and `Cpu` respectively.
* **Gpio**: Add GPIO as a temporary (deprecated) type alias for `Gpio`.

## 0.1.3 (May 27, 2017)

* **DeviceInfo**: Add additional revision codes for old models.
* **GPIO**: Always try `/dev/mem` after `/dev/gpiomem` fails. Return new error `PermissionDenied` when both `/dev/gpiomem` and `/dev/mem` have permission issues. This is a workaround for Ubuntu Core 16 where `/dev/gpiomem` can't be accessed by applications installed using snap (reported by @VBota1).

## 0.1.2 (March 3, 2017)

* **DeviceInfo**: Change returned `u32` references to copied values.

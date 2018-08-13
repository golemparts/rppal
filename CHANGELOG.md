# Changelog

## 0.8.0 (August 13, 2018)

* Gpio: Replace GPIO sysfs interface (/sys/class/gpio) for interrupts with GPIO character device (/dev/gpiochipN).
* Pwm: Add support for up to two hardware PWM channels with configurable period/frequency, duty cycle and polarity.
* Spi: Fix 0-length transfers caused by TransferSegment instances with write_buffer set to None.

## 0.7.1 (June 26, 2018)

* Reverted the use of the recently stabilized Duration::subsec_millis() back to Duration::subsec_nanos() to allow older stable versions of the compiler to work.

## 0.7.0 (June 26, 2018)

* DeviceInfo: (Breaking change) Remove Error::CantAccessProcCpuInfo.
* DeviceInfo: Add additional options to automatically identify the Pi model when /proc/cpuinfo contains inaccurate data.
* Gpio: (Breaking change) Remove Error::ChannelDisconnected.
* I2c: Add support for I2C with basic read/write, block read/write, and write_read.
* I2c: Add support for SMBus with Quick Command, Send/Receive Byte, Read/Write Byte/Word, Process Call, Block Write, and PEC.
* Reduce external dependencies.

## 0.6.0 (June 1, 2018)

* DeviceInfo: (Breaking change) Return model and soc by value, rather than by reference.
* DeviceInfo: (Breaking change) Remove SoC::Bcm2837 to reduce ambiguity. The Pi 3B and Compute Module 3 now return the more accurate SoC::Bcm2837A1.
* DeviceInfo: (Breaking change) Remove SoC::Unknown. An unknown SoC is now treated as a failure.
* DeviceInfo: Return the actual SoC based on the Raspberry Pi model, rather than the inaccurate /proc/cpuinfo data.
* Gpio: (Breaking change) Remove InterruptError. Merge remaining errors with Error.
* Gpio: (Breaking change) Replace all DevGpioMem and DevMem errors with Error::PermissionDenied and Error::Io.
* Gpio: (Breaking change) Change the return value for poll_interrupt() and poll_interrupts() to Ok(Option) on success, with Some() indicating an interrupt triggered, and None indicating a timeout occurred.
* Gpio: (Breaking change) Only a single instance of Gpio can exist at any time. Creating another instance before the existing one goes out of scope will return an Error::InstanceExists.
* Spi: Add support for SPI with half-duplex reads/writes and full-duplex multi-segment transfers.

## 0.5.1 (May 19, 2018)

* Gpio: Add poll_interrupts(), which waits for multiple synchronous interrupts at the same time.
* Gpio: Add public interface for InterruptError.
* Cleanup documentation.

## 0.5.0 (May 9, 2018)

* DeviceInfo: Add hardcoded Raspberry Pi 3 B+ SoC identifier, rather than relying on inaccurate info from /proc/cpuinfo.
* Gpio: Add support for asynchronous interrupts (set_async_interrupt(), clear_async_interrupt()).
* Gpio: Add support for synchronous interrupts (set_interrupt(), clear_interrupt(), poll_interrupt()).

## 0.4.0 (March 19, 2018)

* Gpio: Replace &mut self with &self where possible.

## 0.3.0 (March 16, 2018)

* DeviceInfo: Add support for Raspberry Pi 3 B+.
* DeviceInfo: Set memory offsets based on model info rather than SoC.

## 0.2.0 (October 6, 2017)

* To adhere to Rust's naming conventions, several structs and enums that had GPIO, IO, BCM or CPU somewhere in their name have been changed to Gpio, Io, Bcm and Cpu respectively.
* GPIO has been added as a temporary (deprecated) type alias for Gpio.
* Minor version bump due to incompatible API changes in a 0.x.x release.

## 0.1.3 (May 27, 2017)

* DeviceInfo: Add additional revision codes for old models.
* GPIO: Always try /dev/mem after /dev/gpiomem fails. Return new error PermissionDenied when both /dev/gpiomem and /dev/mem have permission issues. This is a workaround for Ubuntu Core 16 where /dev/gpiomem can't be accessed by applications installed using snap. Reported by VBota1.

## 0.1.2 (March 3, 2017)

* DeviceInfo: Change returned u32 references to copied values.

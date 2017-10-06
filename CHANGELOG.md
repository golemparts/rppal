# 0.2.0 (Oct 6, 2017)

* To adhere to Rust's naming conventions, several structs and enums that
had GPIO, IO, BCM or CPU somewhere in their name have been changed to
Gpio, Io, Bcm and Cpu respectively.
* GPIO has been added as a temporary (deprecated) type alias for Gpio. 
* Minor version bump due to incompatible API changes in a 0.x.x release.

# 0.1.3 (May 27, 2017)

* GPIO: Always try /dev/mem after /dev/gpiomem fails. Return new error PermissionDenied when both /dev/gpiomem and /dev/mem have permission issues. This is a workaround for Ubuntu Core 16 where /dev/gpiomem can't be accessed by applications installed using snap. Reported by VBota1.
* DeviceInfo: Add additional revision codes for old models

# 0.1.2 (March 3, 2017)

* DeviceInfo: Change returned u32 references to copied values

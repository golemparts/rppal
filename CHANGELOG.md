# 0.1.3 (May 27, 2017)

* GPIO: Always try /dev/mem after /dev/gpiomem fails. Return new error PermissionDenied when both /dev/gpiomem and /dev/mem have permission issues. This is a workaround for Ubuntu Core 16 where /dev/gpiomem can't be accessed by applications installed using snap. Reported by VBota1.
* DeviceInfo: Add additional revision codes for old models

# 0.1.2 (March 3, 2017)

* DeviceInfo: Change returned u32 references to copied values

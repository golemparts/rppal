use std::fmt;
use std::fs::OpenOptions;
use std::io;
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::io::AsRawFd;
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use libc::{self, c_void, off_t, size_t, MAP_FAILED, MAP_SHARED, O_SYNC, PROT_READ, PROT_WRITE};

use crate::gpio::{Error, Level, Mode, PullUpDown, Result};
use crate::system::{DeviceInfo, SoC};

const PATH_DEV_GPIOMEM: &str = "/dev/gpiomem";
const PATH_DEV_MEM: &str = "/dev/mem";
// The BCM2835 has 41 32-bit registers related to the GPIO (datasheet @ 6.1).
// The BCM2711 (RPi4) has GPIO-related 32-bit registers #0 .. #60, an address space of 61 registers (datasheet @ 5.1).
const GPIO_MEM_REGISTERS: usize = 61;
const GPIO_MEM_SIZE: usize = GPIO_MEM_REGISTERS * std::mem::size_of::<u32>();
const GPFSEL0: usize = 0x00;
const GPSET0: usize = 0x1c / std::mem::size_of::<u32>();
const GPCLR0: usize = 0x28 / std::mem::size_of::<u32>();
const GPLEV0: usize = 0x34 / std::mem::size_of::<u32>();
const GPPUD: usize = 0x94 / std::mem::size_of::<u32>();
const GPPUDCLK0: usize = 0x98 / std::mem::size_of::<u32>();
// Only available in BCM2711 (RPi4).
const GPPUD_CNTRL_REG0: usize = 0xe4 / std::mem::size_of::<u32>();

pub struct GpioMem {
    mem_ptr: *mut u32,
    locks: [AtomicBool; GPIO_MEM_REGISTERS],
    soc: SoC,
}

impl fmt::Debug for GpioMem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GpioMem")
            .field("mem_ptr", &self.mem_ptr)
            .field("locks", &format_args!("{{ .. }}"))
            .field("soc", &self.soc)
            .finish()
    }
}

impl GpioMem {
    pub fn open() -> Result<GpioMem> {
        // Try /dev/gpiomem first. If that fails, try /dev/mem instead. If neither works,
        // report back the error that's the most relevant.
        let mem_ptr = match Self::map_devgpiomem() {
            Ok(ptr) => ptr,
            Err(gpiomem_err) => match Self::map_devmem() {
                Ok(ptr) => ptr,
                Err(Error::Io(ref e)) if e.kind() == io::ErrorKind::PermissionDenied => {
                    // Did /dev/gpiomem also give us a Permission Denied error? If so, return
                    // that path instead of /dev/mem. Solving /dev/gpiomem issues should be
                    // preferred (add user to gpio group) over /dev/mem (use sudo),
                    match gpiomem_err {
                        Error::Io(ref e) if e.kind() == io::ErrorKind::PermissionDenied => {
                            return Err(Error::PermissionDenied(String::from(PATH_DEV_GPIOMEM)));
                        }
                        _ => return Err(Error::PermissionDenied(String::from(PATH_DEV_MEM))),
                    }
                }
                Err(Error::UnknownModel) => return Err(Error::UnknownModel),
                _ => return Err(gpiomem_err),
            },
        };

        let locks = init_array!(AtomicBool::new(false), GPIO_MEM_REGISTERS);

        // Identify which SoC we're using.
        let soc = DeviceInfo::new().map_err(|_| Error::UnknownModel)?.soc();

        Ok(GpioMem {
            mem_ptr,
            locks,
            soc,
        })
    }

    fn map_devgpiomem() -> Result<*mut u32> {
        // Open /dev/gpiomem with read/write/sync flags. This might fail if
        // /dev/gpiomem doesn't exist (< Raspbian Jessie), or /dev/gpiomem
        // doesn't have the appropriate permissions, or the current user is
        // not a member of the gpio group.
        let gpiomem_file = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(O_SYNC)
            .open(PATH_DEV_GPIOMEM)?;

        // Memory-map /dev/gpiomem at offset 0
        let gpiomem_ptr = unsafe {
            libc::mmap(
                ptr::null_mut(),
                GPIO_MEM_SIZE,
                PROT_READ | PROT_WRITE,
                MAP_SHARED,
                gpiomem_file.as_raw_fd(),
                0,
            )
        };

        if gpiomem_ptr == MAP_FAILED {
            return Err(Error::Io(io::Error::last_os_error()));
        }

        Ok(gpiomem_ptr as *mut u32)
    }

    fn map_devmem() -> Result<*mut u32> {
        // Identify which SoC we're using, so we know what offset to start at
        let device_info = DeviceInfo::new().map_err(|_| Error::UnknownModel)?;

        let mem_file = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(O_SYNC)
            .open(PATH_DEV_MEM)?;

        // Memory-map /dev/mem at the appropriate offset for our SoC
        let mem_ptr = unsafe {
            libc::mmap(
                ptr::null_mut(),
                GPIO_MEM_SIZE,
                PROT_READ | PROT_WRITE,
                MAP_SHARED,
                mem_file.as_raw_fd(),
                (device_info.peripheral_base() + device_info.gpio_offset()) as off_t,
            )
        };

        if mem_ptr == MAP_FAILED {
            return Err(Error::Io(io::Error::last_os_error()));
        }

        Ok(mem_ptr as *mut u32)
    }

    #[inline(always)]
    fn read(&self, offset: usize) -> u32 {
        unsafe { ptr::read_volatile(self.mem_ptr.add(offset)) }
    }

    #[inline(always)]
    fn write(&self, offset: usize, value: u32) {
        unsafe {
            ptr::write_volatile(self.mem_ptr.add(offset), value);
        }
    }

    #[inline(always)]
    pub(crate) fn set_high(&self, pin: u8) {
        let offset = GPSET0 + pin as usize / 32;
        let shift = pin % 32;

        self.write(offset, 1 << shift);
    }

    #[inline(always)]
    pub(crate) fn set_low(&self, pin: u8) {
        let offset = GPCLR0 + pin as usize / 32;
        let shift = pin % 32;

        self.write(offset, 1 << shift);
    }

    #[inline(always)]
    pub(crate) fn level(&self, pin: u8) -> Level {
        let offset = GPLEV0 + pin as usize / 32;
        let shift = pin % 32;
        let reg_value = self.read(offset);

        unsafe { std::mem::transmute((reg_value >> shift) as u8 & 0b1) }
    }

    pub(crate) fn mode(&self, pin: u8) -> Mode {
        let offset = GPFSEL0 + pin as usize / 10;
        let shift = (pin % 10) * 3;
        let reg_value = self.read(offset);

        unsafe { std::mem::transmute((reg_value >> shift) as u8 & 0b111) }
    }

    pub(crate) fn set_mode(&self, pin: u8, mode: Mode) {
        let offset = GPFSEL0 + pin as usize / 10;
        let shift = (pin % 10) * 3;

        loop {
            if self.locks[offset]
                .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                break;
            }
        }

        let reg_value = self.read(offset);
        self.write(
            offset,
            (reg_value & !(0b111 << shift)) | ((mode as u32) << shift),
        );

        self.locks[offset].store(false, Ordering::SeqCst);
    }

    pub(crate) fn set_pullupdown(&self, pin: u8, pud: PullUpDown) {
        // Offset for register.
        let offset: usize;
        // Bit shift for pin position within register value.
        let shift: u8;

        // Only BCM2711 (RPi4) needs special handling for now.
        if self.soc == SoC::Bcm2711 {
            offset = GPPUD_CNTRL_REG0 + pin as usize / 16;
            shift = pin % 16 * 2;

            // Index for lock is different than register.
            let lock = GPPUD_CNTRL_REG0 + pin as usize / 32;

            // Pull up vs pull down has a reverse bit pattern on BCM2711 vs others.
            let pud = match pud {
                PullUpDown::Off => 0b00u32,
                PullUpDown::PullDown => 0b10,
                PullUpDown::PullUp => 0b01,
            };

            loop {
                if self.locks[lock]
                    .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok()
                {
                    break;
                }
            }

            let reg_value = self.read(offset);
            self.write(offset, (reg_value & !(0b11 << shift)) | (pud << shift));

            self.locks[lock].store(false, Ordering::SeqCst);
        } else {
            offset = GPPUDCLK0 + pin as usize / 32;
            shift = pin % 32;

            loop {
                if self.locks[GPPUD]
                    .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok()
                {
                    if self.locks[offset]
                        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                        .is_ok()
                    {
                        break;
                    } else {
                        self.locks[GPPUD].store(false, Ordering::SeqCst);
                    }
                }
            }

            // Set the control signal in GPPUD.
            let reg_value = self.read(GPPUD);
            self.write(GPPUD, (reg_value & !0b11) | ((pud as u32) & 0b11));

            // The datasheet mentions waiting at least 150 cycles for set-up and hold, but
            // doesn't state which clock is used. This is likely the VPU clock (see
            // https://www.raspberrypi.org/forums/viewtopic.php?f=72&t=163352). At either
            // 250MHz or 400MHz, a 5µs delay + overhead is more than adequate.

            // Set-up time for the control signal. >= 5µs
            thread::sleep(Duration::new(0, 5000));
            // Clock the control signal into the selected pin.
            self.write(offset, 1 << shift);

            // Hold time for the control signal. >= 5µs
            thread::sleep(Duration::new(0, 5000));
            // Remove the control signal and clock.
            self.write(GPPUD, reg_value & !0b11);
            self.write(offset, 0);

            self.locks[offset].store(false, Ordering::SeqCst);
            self.locks[GPPUD].store(false, Ordering::SeqCst);
        }
    }
}

impl Drop for GpioMem {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.mem_ptr as *mut c_void, GPIO_MEM_SIZE as size_t);
        }
    }
}

// Required because of the raw pointer to our memory-mapped file
unsafe impl Send for GpioMem {}

unsafe impl Sync for GpioMem {}

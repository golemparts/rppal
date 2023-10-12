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

const PATH_DEV_GPIOMEM: &str = "/dev/rp1-gpiomem";

const RW_OFFSET: usize = 0x0000;
const XOR_OFFSET: usize = 0x1000;
const SET_OFFSET: usize = 0x2000;
const CLR_OFFSET: usize = 0x3000;

const GPIO_STATUS: usize = 0x0000;
const GPIO_CTRL: usize = 0x0004;

// rp1-gpiomem contains IO_BANK0-2, SYS_RIO0-2, PADS_BANK0-2, PADS_ETH
const MEM_SIZE: usize = 0x30000 * std::mem::size_of::<u32>();

// We'll only be working with IO_BANK0 and PADS_BANK0
const IO_BANK0_OFFSET: usize = 0x00;
const PADS_BANK0_OFFSET: usize = 0x20000;

pub struct GpioMem {
    mem_ptr: *mut u32,
    soc: SoC,
}

impl fmt::Debug for GpioMem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GpioMem")
            .field("mem_ptr", &self.mem_ptr)
            .field("soc", &self.soc)
            .finish()
    }
}

impl GpioMem {
    pub fn open() -> Result<GpioMem> {
        let mem_ptr = Self::map_devgpiomem()?;

        // Identify which SoC we're using.
        let soc = DeviceInfo::new().map_err(|_| Error::UnknownModel)?.soc();

        Ok(GpioMem { mem_ptr, soc })
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

        // Memory-map /dev/rp1-gpiomem at offset 0
        let gpiomem_ptr = unsafe {
            libc::mmap(
                ptr::null_mut(),
                MEM_SIZE,
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
        unimplemented!()
    }

    #[inline(always)]
    pub(crate) fn set_low(&self, pin: u8) {
        unimplemented!()
    }

    #[inline(always)]
    pub(crate) fn level(&self, pin: u8) -> Level {
        unimplemented!()
    }

    pub(crate) fn mode(&self, pin: u8) -> Mode {
        unimplemented!()
    }

    pub(crate) fn set_mode(&self, pin: u8, mode: Mode) {
        unimplemented!()
    }

    pub(crate) fn set_pullupdown(&self, pin: u8, pud: PullUpDown) {
        unimplemented!()
    }
}

impl Drop for GpioMem {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.mem_ptr as *mut c_void, MEM_SIZE as size_t);
        }
    }
}

// Required because of the raw pointer to our memory-mapped file
unsafe impl Send for GpioMem {}

unsafe impl Sync for GpioMem {}

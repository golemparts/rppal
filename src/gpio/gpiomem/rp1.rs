#![allow(dead_code)]

use std::fmt;
use std::fs::OpenOptions;
use std::io;
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::io::AsRawFd;
use std::ptr;

use libc::{self, c_void, size_t, MAP_FAILED, MAP_SHARED, O_SYNC, PROT_READ, PROT_WRITE};

use crate::gpio::{Bias, Error, Level, Mode, Result};
use crate::system::{DeviceInfo, SoC};

use super::GpioRegisters;

const PATH_DEV_GPIOMEM: &str = "/dev/gpiomem0";

// Each register contains 32 bits
const REG_SIZE: usize = std::mem::size_of::<u32>();
// rp1-gpiomem contains IO_BANK0-2, SYS_RIO0-2, PADS_BANK0-2, PADS_ETH
const MEM_SIZE: usize = 0x30000;

// We're only accessing the first 28 GPIOs. The rest is currently marked
// as internal-use only, so we only need IO_BANK0/SYS_RIO0/PADS_BANK0.
const IO_BANK0_OFFSET: usize = 0x00000;
const SYS_RIO0_OFFSET: usize = 0x10000;
const PADS_BANK0_OFFSET: usize = 0x20000;

// Atomic register access (datasheet @ 2.4)
const RW_OFFSET: usize = 0x0000;
const XOR_OFFSET: usize = 0x1000;
const SET_OFFSET: usize = 0x2000;
const CLR_OFFSET: usize = 0x3000;

// STATUS and CTRL offsets for the IO_BANK registers (datasheet @ 3.1.4)
const GPIO_STATUS: usize = 0x0000;
const GPIO_CTRL: usize = 0x0004;
// Offset to the next GPIO for the IO_BANK registers (datasheet @ 3.1.4)
const GPIO_OFFSET: usize = 8;

const CTRL_FUNCSEL_MASK: u32 = 0x001f;
const CTRL_FUNCSEL_LSB: u32 = 0;
const CTRL_OUTOVER_MASK: u32 = 0x3000;
const CTRL_OUTOVER_LSB: u32 = 12;
const CTRL_OEOVER_MASK: u32 = 0xc000;
const CTRL_OEOVER_LSB: u32 = 14;

// Drive output from peripheral signal selected by FUNCSEL
const OUTOVER_PERI: u32 = 0;
// Drive output enable from peripheral signal selected by FUNCSEL
const OEOVER_PERI: u32 = 0;

// Function select modes
const FSEL_ALT0: u8 = 0;
const FSEL_ALT1: u8 = 1;
const FSEL_ALT2: u8 = 2;
const FSEL_ALT3: u8 = 3;
const FSEL_ALT4: u8 = 4;
const FSEL_ALT5: u8 = 5; // GPIO
const FSEL_ALT6: u8 = 6;
const FSEL_ALT7: u8 = 7;
const FSEL_ALT8: u8 = 8;

// GPIO offset for the PADS_BANK registers (datasheet @ 3.1.4)
const PADS_GPIO: usize = 0x04;
// Offset to the next GPIO for the PADS_BANK registers (datasheet @ 3.1.4)
const PADS_OFFSET: usize = 4;

const PADS_IN_ENABLE_MASK: u32 = 0x40;
const PADS_OUT_DISABLE_MASK: u32 = 0x80;

const PADS_BIAS_MASK: u32 = 0x0c;
const PADS_BIAS_LSB: u32 = 2;

const PADS_BIAS_OFF: u32 = 0;
const PADS_BIAS_DOWN: u32 = 1;
const PADS_BIAS_UP: u32 = 2;

// GPIO output drive
const RIO_OUT: usize = 0x00;
// GPIO output drive enable
const RIO_OE: usize = 0x04;
// GPIO input value
const RIO_IN: usize = 0x08;

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
        // Open /dev/rp1-gpiomem with read/write/sync flags. This might fail if
        // /dev/rp1-gpiomem doesn't exist (< Raspbian Jessie), or /dev/rp1-gpiomem
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

    fn direction(&self, pin: u8) -> Mode {
        let offset = (SYS_RIO0_OFFSET + RIO_OE) / REG_SIZE;
        let reg_value = (self.read(offset) >> pin) as u8 & 0b1;

        if reg_value > 0 {
            Mode::Output
        } else {
            Mode::Input
        }
    }

    fn set_direction(&self, pin: u8, mode: Mode) {
        let offset = match mode {
            Mode::Output => (SYS_RIO0_OFFSET + RIO_OE + SET_OFFSET) / REG_SIZE,
            _ => (SYS_RIO0_OFFSET + RIO_OE + CLR_OFFSET) / REG_SIZE,
        };

        self.write(offset, 1 << pin);
    }

    fn input_enable(&self, pin: u8) {
        let offset =
            (PADS_BANK0_OFFSET + PADS_GPIO + (pin as usize * PADS_OFFSET) + SET_OFFSET) / REG_SIZE;

        self.write(offset, PADS_IN_ENABLE_MASK);
    }

    fn output_enable(&self, pin: u8) {
        let offset =
            (PADS_BANK0_OFFSET + PADS_GPIO + (pin as usize * PADS_OFFSET) + CLR_OFFSET) / REG_SIZE;

        self.write(offset, PADS_OUT_DISABLE_MASK);
    }
}

impl GpioRegisters for GpioMem {
    #[inline(always)]
    fn set_high(&self, pin: u8) {
        let offset = (SYS_RIO0_OFFSET + RIO_OUT + SET_OFFSET) / REG_SIZE;

        self.write(offset, 1 << pin);
    }

    #[inline(always)]
    fn set_low(&self, pin: u8) {
        let offset = (SYS_RIO0_OFFSET + RIO_OUT + CLR_OFFSET) / REG_SIZE;

        self.write(offset, 1 << pin);
    }

    #[inline(always)]
    fn level(&self, pin: u8) -> Level {
        let offset = (SYS_RIO0_OFFSET + RIO_IN) / REG_SIZE;
        let reg_value = self.read(offset);

        unsafe { std::mem::transmute((reg_value >> pin) as u8 & 0b1) }
    }

    fn mode(&self, pin: u8) -> Mode {
        let offset =
            (IO_BANK0_OFFSET + GPIO_CTRL + (pin as usize * GPIO_OFFSET) + RW_OFFSET) / REG_SIZE;
        let reg_value = self.read(offset);

        match (reg_value & CTRL_FUNCSEL_MASK) as u8 {
            FSEL_ALT0 => Mode::Alt0,
            FSEL_ALT1 => Mode::Alt1,
            FSEL_ALT2 => Mode::Alt2,
            FSEL_ALT3 => Mode::Alt3,
            FSEL_ALT4 => Mode::Alt4,
            FSEL_ALT5 => self.direction(pin), // GPIO
            FSEL_ALT6 => Mode::Alt6,
            FSEL_ALT7 => Mode::Alt7,
            FSEL_ALT8 => Mode::Alt8,
            _ => Mode::Input,
        }
    }

    fn set_mode(&self, pin: u8, mode: Mode) {
        self.input_enable(pin);
        self.output_enable(pin);

        let offset =
            (IO_BANK0_OFFSET + GPIO_CTRL + (pin as usize * GPIO_OFFSET) + RW_OFFSET) / REG_SIZE;
        let mut reg_value = self.read(offset);

        let fsel_mode = match mode {
            Mode::Input => FSEL_ALT5,  // GPIO
            Mode::Output => FSEL_ALT5, // GPIO
            Mode::Alt0 => FSEL_ALT0,
            Mode::Alt1 => FSEL_ALT1,
            Mode::Alt2 => FSEL_ALT2,
            Mode::Alt3 => FSEL_ALT3,
            Mode::Alt4 => FSEL_ALT4,
            Mode::Alt5 => FSEL_ALT5,
            Mode::Alt6 => FSEL_ALT6,
            Mode::Alt7 => FSEL_ALT7,
            Mode::Alt8 => FSEL_ALT8,
        };

        // Set the actual direction here, since this isn't set in the FSEL register.
        if mode == Mode::Input || mode == Mode::Output {
            self.set_direction(pin, mode);
        }

        reg_value = (reg_value & !CTRL_OUTOVER_MASK) | (OUTOVER_PERI << CTRL_OUTOVER_LSB);
        reg_value = (reg_value & !CTRL_OEOVER_MASK) | (OEOVER_PERI << CTRL_OEOVER_LSB);
        reg_value = (reg_value & !CTRL_FUNCSEL_MASK) | ((fsel_mode as u32) << CTRL_FUNCSEL_LSB);

        self.write(offset, reg_value);
    }

    fn set_bias(&self, pin: u8, bias: Bias) {
        let offset =
            (PADS_BANK0_OFFSET + PADS_GPIO + (pin as usize * PADS_OFFSET) + RW_OFFSET) / REG_SIZE;
        let mut reg_value = self.read(offset);

        reg_value = match bias {
            Bias::Off => (reg_value & !PADS_BIAS_MASK) | (PADS_BIAS_OFF << PADS_BIAS_LSB),
            Bias::PullDown => (reg_value & !PADS_BIAS_MASK) | (PADS_BIAS_DOWN << PADS_BIAS_LSB),
            Bias::PullUp => (reg_value & !PADS_BIAS_MASK) | (PADS_BIAS_UP << PADS_BIAS_LSB),
        };

        self.write(offset, reg_value);
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

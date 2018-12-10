use std::sync::{Arc, Mutex};

use crate::gpio::{Mode, Level, GPIO_OFFSET_GPLEV, GPIO_OFFSET_GPFSEL, GPIO_OFFSET_GPCLR, GPIO_OFFSET_GPSET, mem::GpioMem};

#[derive(Debug)]
pub struct Pin {
    pin: u8,
    gpio_mem: Arc<Mutex<GpioMem>>,
}

impl Pin {
    pub(crate) fn new(pin: u8, gpio_mem: Arc<Mutex<GpioMem>>) -> Pin {
        Pin { pin, gpio_mem }
    }

    pub fn as_input(&mut self) -> InputPin {
        InputPin::new(self)
    }

    pub fn as_output(&mut self) -> OutputPin {
        OutputPin::new(self, Mode::Output)
    }

    pub fn as_output_with_mode(&mut self, mode: Mode) -> OutputPin {
        OutputPin::new(self, mode)
    }

    pub(crate) fn set_mode(&mut self, mode: Mode) {
        let reg_addr: usize = GPIO_OFFSET_GPFSEL + (self.pin / 10) as usize;

        let reg_value = (*self.gpio_mem.lock().unwrap()).read(reg_addr);
        (*self.gpio_mem.lock().unwrap()).write(
            reg_addr,
            (reg_value & !(0b111 << ((self.pin % 10) * 3)))
                | ((mode as u32 & 0b111) << ((self.pin % 10) * 3)),
        );

    }

    /// Returns the current GPIO pin mode.
    pub fn mode(&self) -> Mode {
        let reg_addr: usize = GPIO_OFFSET_GPFSEL + (self.pin / 10) as usize;
        let reg_value = (*self.gpio_mem.lock().unwrap()).read(reg_addr);
        let mode_value = ((reg_value >> ((self.pin % 10) * 3)) & 0b111) as u8;

        mode_value.into()
    }
}

#[derive(Debug)]
pub struct InputPin<'a> {
    pin: &'a mut Pin,
    prev_mode: Option<Mode>,
}

impl<'a> InputPin<'a> {
    pub(crate) fn new(pin: &'a mut Pin) -> InputPin<'a> {
        let prev_mode = pin.mode();

        let prev_mode = if prev_mode == Mode::Input {
            None
        } else {
            pin.set_mode(Mode::Input);
            Some(prev_mode)
        };

        InputPin { pin, prev_mode }
    }

    pub fn read(&self) -> Level {
        let reg_addr: usize = GPIO_OFFSET_GPLEV + (self.pin.pin / 32) as usize;
        let reg_value = (*self.pin.gpio_mem.lock().unwrap()).read(reg_addr);

        if (reg_value & (1 << (self.pin.pin % 32))) > 0 {
            Level::High
        } else {
            Level::Low
        }
    }
}

impl<'a> Drop for InputPin<'a> {
    fn drop(&mut self) {
        if let Some(prev_mode) = self.prev_mode {
            self.pin.set_mode(prev_mode)
        }
    }
}

#[derive(Debug)]
pub struct OutputPin<'a> {
    pin: &'a mut Pin,
    mode: Mode,
    prev_mode: Option<Mode>,
}

impl<'a> OutputPin<'a> {
    pub(crate) fn new(pin: &'a mut Pin, mode: Mode) -> OutputPin<'a> {
        let prev_mode = pin.mode();

        let prev_mode = if prev_mode == mode {
            None
        } else {
            pin.set_mode(mode);
            Some(prev_mode)
        };

        OutputPin { pin, mode, prev_mode }
    }

    pub fn set_low(&mut self) {
        self.write(Level::Low)
    }

    pub fn set_high(&mut self) {
        self.write(Level::High)
    }

    pub fn write(&mut self, level: Level) {
        let reg_addr: usize = match level {
            Level::Low => GPIO_OFFSET_GPCLR + (self.pin.pin / 32) as usize,
            Level::High => GPIO_OFFSET_GPSET + (self.pin.pin / 32) as usize,
        };

        (*self.pin.gpio_mem.lock().unwrap()).write(reg_addr, 1 << (self.pin.pin % 32));
    }
}

impl<'a> Drop for OutputPin<'a> {
  fn drop(&mut self) {
    if let Some(prev_mode) = self.prev_mode {
      self.pin.set_mode(prev_mode)
    }
  }
}

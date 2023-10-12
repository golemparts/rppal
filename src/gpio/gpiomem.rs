use crate::gpio::{Level, Mode, PullUpDown};

pub mod bcm;
pub mod rp1;

pub(crate) trait GpioRegisters: std::fmt::Debug + Sync + Send {
    fn set_high(&self, pin: u8);
    fn set_low(&self, pin: u8);
    fn level(&self, pin: u8) -> Level;
    fn mode(&self, pin: u8) -> Mode;
    fn set_mode(&self, pin: u8, mode: Mode);
    fn set_pullupdown(&self, pin: u8, pud: PullUpDown);
}

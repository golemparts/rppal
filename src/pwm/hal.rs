use super::Pwm;

/// `PwmPin` trait implementation for `embedded-hal` v0.2.6.
impl embedded_hal_0::PwmPin for Pwm {
    type Duty = f64;

    fn disable(&mut self) {
        let _ = Pwm::disable(self);
    }

    fn enable(&mut self) {
        let _ = Pwm::enable(self);
    }

    fn get_duty(&self) -> Self::Duty {
        self.duty_cycle().unwrap_or_default()
    }

    fn get_max_duty(&self) -> Self::Duty {
        1.0
    }

    fn set_duty(&mut self, duty: Self::Duty) {
        let _ = self.set_duty_cycle(duty);
    }
}

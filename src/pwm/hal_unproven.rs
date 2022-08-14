use std::time::Duration;

use super::Pwm;

/// Unproven `Pwm` trait implementation for `embedded-hal` v0.2.6.
impl embedded_hal_0::Pwm for Pwm {
    type Duty = f64;
    type Channel = ();
    type Time = Duration;

    /// Disables a PWM `channel`.
    fn disable(&mut self, _channel: Self::Channel) {
        let _ = Pwm::disable(self);
    }

    /// Enables a PWM `channel`.
    fn enable(&mut self, _channel: Self::Channel) {
        let _ = Pwm::enable(self);
    }

    /// Returns the current PWM period.
    fn get_period(&self) -> Self::Time {
        self.period().unwrap_or_default()
    }

    /// Returns the current duty cycle.
    fn get_duty(&self, _channel: Self::Channel) -> Self::Duty {
        self.duty_cycle().unwrap_or_default()
    }

    /// Returns the maximum duty cycle value.
    fn get_max_duty(&self) -> Self::Duty {
        1.0
    }

    /// Sets a new duty cycle.
    fn set_duty(&mut self, _channel: Self::Channel, duty: Self::Duty) {
        let _ = self.set_duty_cycle(duty);
    }

    /// Sets a new PWM period.
    fn set_period<P>(&mut self, period: P)
    where
        P: Into<Self::Time>,
    {
        let _ = Pwm::set_period(self, period.into());
    }
}

use core::convert::Infallible;
use std::time::Duration;

use embedded_hal::digital::{
    InputPin as InputPinHal, StatefulOutputPin as StatefulOutputPinHal,
    ToggleableOutputPin as ToggleableOutputPinHal,
};

use super::{InputPin, IoPin, OutputPin, Pin};
use crate::gpio::Mode;

const NANOS_PER_SEC: f64 = 1_000_000_000.0;

/// Unproven `InputPin` trait implementation for `embedded-hal` v0.2.6.
impl embedded_hal_0::digital::v2::InputPin for Pin {
    type Error = Infallible;

    fn is_high(&self) -> Result<bool, Self::Error> {
        InputPinHal::is_high(self)
    }

    fn is_low(&self) -> Result<bool, Self::Error> {
        InputPinHal::is_low(self)
    }
}

/// Unproven `InputPin` trait implementation for `embedded-hal` v0.2.6.
impl embedded_hal_0::digital::v2::InputPin for InputPin {
    type Error = Infallible;

    fn is_high(&self) -> Result<bool, Self::Error> {
        InputPinHal::is_high(self)
    }

    fn is_low(&self) -> Result<bool, Self::Error> {
        InputPinHal::is_low(self)
    }
}

/// Unproven `InputPin` trait implementation for `embedded-hal` v0.2.6.
impl embedded_hal_0::digital::v2::InputPin for IoPin {
    type Error = Infallible;

    fn is_high(&self) -> Result<bool, Self::Error> {
        InputPinHal::is_high(self)
    }

    fn is_low(&self) -> Result<bool, Self::Error> {
        InputPinHal::is_low(self)
    }
}

/// Unproven `InputPin` trait implementation for `embedded-hal` v0.2.6.
impl embedded_hal_0::digital::v2::InputPin for OutputPin {
    type Error = Infallible;

    fn is_high(&self) -> Result<bool, Self::Error> {
        InputPinHal::is_high(self)
    }

    fn is_low(&self) -> Result<bool, Self::Error> {
        InputPinHal::is_low(self)
    }
}

/// Unproven `StatefulOutputPin` trait implementation for `embedded-hal` v0.2.6.
impl embedded_hal_0::digital::v2::StatefulOutputPin for IoPin {
    fn is_set_high(&self) -> Result<bool, Self::Error> {
        StatefulOutputPinHal::is_set_high(self)
    }

    fn is_set_low(&self) -> Result<bool, Self::Error> {
        StatefulOutputPinHal::is_set_low(self)
    }
}

/// Unproven `StatefulOutputPin` trait implementation for `embedded-hal` v0.2.6.
impl embedded_hal_0::digital::v2::StatefulOutputPin for OutputPin {
    fn is_set_high(&self) -> Result<bool, Self::Error> {
        StatefulOutputPinHal::is_set_high(self)
    }

    fn is_set_low(&self) -> Result<bool, Self::Error> {
        StatefulOutputPinHal::is_set_low(self)
    }
}

/// Unproven `ToggleableOutputPin` trait implementation for `embedded-hal` v0.2.6.
impl embedded_hal_0::digital::v2::ToggleableOutputPin for IoPin {
    type Error = Infallible;

    fn toggle(&mut self) -> Result<(), Self::Error> {
        ToggleableOutputPinHal::toggle(self)
    }
}

/// Unproven `ToggleableOutputPin` trait implementation for `embedded-hal` v0.2.6.
impl embedded_hal_0::digital::v2::ToggleableOutputPin for OutputPin {
    type Error = Infallible;

    fn toggle(&mut self) -> Result<(), Self::Error> {
        ToggleableOutputPinHal::toggle(self)
    }
}

/// Unproven `Pwm` trait implementation for `embedded-hal` v0.2.6.
impl embedded_hal_0::Pwm for OutputPin {
    type Duty = f64;
    type Channel = ();
    type Time = Duration;

    /// Disables a PWM `channel`.
    fn disable(&mut self, _channel: Self::Channel) {
        let _ = self.clear_pwm();
    }

    /// Enables a PWM `channel`.
    fn enable(&mut self, _channel: Self::Channel) {
        let _ = self.set_pwm_frequency(self.frequency, self.duty_cycle);
    }

    /// Returns the current PWM period.
    fn get_period(&self) -> Self::Time {
        Duration::from_nanos(if self.frequency == 0.0 {
            0
        } else {
            ((1.0 / self.frequency) * NANOS_PER_SEC) as u64
        })
    }

    /// Returns the current duty cycle.
    fn get_duty(&self, _channel: Self::Channel) -> Self::Duty {
        self.duty_cycle
    }

    /// Returns the maximum duty cycle value.
    fn get_max_duty(&self) -> Self::Duty {
        1.0
    }

    /// Sets a new duty cycle.
    fn set_duty(&mut self, _channel: Self::Channel, duty: Self::Duty) {
        self.duty_cycle = duty.max(0.0).min(1.0);

        if self.soft_pwm.is_some() {
            let _ = self.set_pwm_frequency(self.frequency, self.duty_cycle);
        }
    }

    /// Sets a new PWM period.
    fn set_period<P>(&mut self, period: P)
    where
        P: Into<Self::Time>,
    {
        let period = period.into();
        self.frequency =
            1.0 / (period.as_secs() as f64 + (f64::from(period.subsec_nanos()) / NANOS_PER_SEC));

        if self.soft_pwm.is_some() {
            let _ = self.set_pwm_frequency(self.frequency, self.duty_cycle);
        }
    }
}

/// Unproven `Pwm` trait implementation for `embedded-hal` v0.2.6.
impl embedded_hal_0::Pwm for IoPin {
    type Duty = f64;
    type Channel = ();
    type Time = Duration;

    /// Disables a PWM `channel`.
    fn disable(&mut self, _channel: Self::Channel) {
        let _ = self.clear_pwm();
    }

    /// Enables a PWM `channel`.
    fn enable(&mut self, _channel: Self::Channel) {
        let _ = self.set_pwm_frequency(self.frequency, self.duty_cycle);
    }

    /// Returns the current PWM period.
    fn get_period(&self) -> Self::Time {
        Duration::from_nanos(if self.frequency == 0.0 {
            0
        } else {
            ((1.0 / self.frequency) * NANOS_PER_SEC) as u64
        })
    }

    /// Returns the current duty cycle.
    fn get_duty(&self, _channel: Self::Channel) -> Self::Duty {
        self.duty_cycle
    }

    /// Returns the maximum duty cycle value.
    fn get_max_duty(&self) -> Self::Duty {
        1.0
    }

    /// Sets a new duty cycle.
    fn set_duty(&mut self, _channel: Self::Channel, duty: Self::Duty) {
        self.duty_cycle = duty.max(0.0).min(1.0);

        if self.soft_pwm.is_some() {
            let _ = self.set_pwm_frequency(self.frequency, self.duty_cycle);
        }
    }

    /// Sets a new PWM period.
    fn set_period<P>(&mut self, period: P)
    where
        P: Into<Self::Time>,
    {
        let period = period.into();
        self.frequency =
            1.0 / (period.as_secs() as f64 + (f64::from(period.subsec_nanos()) / NANOS_PER_SEC));

        if self.soft_pwm.is_some() {
            let _ = self.set_pwm_frequency(self.frequency, self.duty_cycle);
        }
    }
}

/// Unproven `IoPin` trait implementation for `embedded-hal` v0.2.6.
impl embedded_hal_0::digital::v2::IoPin<IoPin, IoPin> for IoPin {
    type Error = Infallible;

    /// Tries to convert this pin to input mode.
    ///
    /// If the pin is already in input mode, this method should succeed.
    fn into_input_pin(mut self) -> Result<IoPin, Self::Error> {
        if self.mode() != Mode::Input {
            self.set_mode(Mode::Input);
        }

        Ok(self)
    }

    /// Tries to convert this pin to output mode with the given initial state.
    ///
    /// If the pin is already in the requested state, this method should
    /// succeed.
    fn into_output_pin(
        mut self,
        state: embedded_hal_0::digital::v2::PinState,
    ) -> Result<IoPin, Self::Error> {
        match state {
            embedded_hal_0::digital::v2::PinState::Low => self.set_low(),
            embedded_hal_0::digital::v2::PinState::High => self.set_high(),
        }

        if self.mode() != Mode::Output {
            self.set_mode(Mode::Output);
        }

        Ok(self)
    }
}

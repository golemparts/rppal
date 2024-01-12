use core::convert::Infallible;
use std::time::Duration;

use super::{InputPin, IoPin, Level, Mode, OutputPin, Pin};

const NANOS_PER_SEC: f64 = 1_000_000_000.0;

#[cfg(feature = "embedded-hal-0")]
impl embedded_hal_0::digital::v2::InputPin for Pin {
    type Error = Infallible;

    fn is_high(&self) -> Result<bool, Self::Error> {
        Ok((*self).read() == Level::High)
    }

    fn is_low(&self) -> Result<bool, Self::Error> {
        Ok((*self).read() == Level::Low)
    }
}

#[cfg(feature = "embedded-hal-0")]
impl embedded_hal_0::digital::v2::InputPin for InputPin {
    type Error = Infallible;

    fn is_high(&self) -> Result<bool, Self::Error> {
        Ok((*self).is_high())
    }

    fn is_low(&self) -> Result<bool, Self::Error> {
        Ok((*self).is_high())
    }
}

#[cfg(feature = "embedded-hal-0")]
impl embedded_hal_0::digital::v2::InputPin for IoPin {
    type Error = Infallible;

    fn is_high(&self) -> Result<bool, Self::Error> {
        Ok((*self).is_high())
    }

    fn is_low(&self) -> Result<bool, Self::Error> {
        Ok((*self).is_high())
    }
}

#[cfg(feature = "embedded-hal-0")]
impl embedded_hal_0::digital::v2::InputPin for OutputPin {
    type Error = Infallible;

    fn is_high(&self) -> Result<bool, Self::Error> {
        Ok((*self).is_set_high())
    }

    fn is_low(&self) -> Result<bool, Self::Error> {
        Ok((*self).is_set_low())
    }
}

#[cfg(feature = "embedded-hal-0")]
impl embedded_hal_0::digital::v2::StatefulOutputPin for IoPin {
    fn is_set_high(&self) -> Result<bool, Self::Error> {
        Ok((*self).is_high())
    }

    fn is_set_low(&self) -> Result<bool, Self::Error> {
        Ok((*self).is_low())
    }
}

#[cfg(feature = "embedded-hal-0")]
impl embedded_hal_0::digital::v2::StatefulOutputPin for OutputPin {
    fn is_set_high(&self) -> Result<bool, Self::Error> {
        Ok((*self).is_set_high())
    }

    fn is_set_low(&self) -> Result<bool, Self::Error> {
        Ok((*self).is_set_low())
    }
}

#[cfg(feature = "embedded-hal-0")]
impl embedded_hal_0::digital::v2::ToggleableOutputPin for IoPin {
    type Error = Infallible;

    fn toggle(&mut self) -> Result<(), Self::Error> {
        embedded_hal::digital::StatefulOutputPin::toggle(self)
    }
}

#[cfg(feature = "embedded-hal-0")]
impl embedded_hal_0::digital::v2::ToggleableOutputPin for OutputPin {
    type Error = Infallible;

    fn toggle(&mut self) -> Result<(), Self::Error> {
        embedded_hal::digital::StatefulOutputPin::toggle(self)
    }
}

#[cfg(feature = "embedded-hal-0")]
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
        self.duty_cycle = duty.clamp(0.0, 1.0);

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

#[cfg(feature = "embedded-hal-0")]
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
        self.duty_cycle = duty.clamp(0.0, 1.0);

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

#[cfg(feature = "embedded-hal-0")]
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

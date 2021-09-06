// Copyright (c) 2017-2021 Rene van der Meer
//
// Permission is hereby granted, free of charge, to any person obtaining a
// copy of this software and associated documentation files (the "Software"),
// to deal in the Software without restriction, including without limitation
// the rights to use, copy, modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software, and to permit persons to whom the
// Software is furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL
// THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
// FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.

use core::convert::Infallible;
use std::time::Duration;

use embedded_hal::digital::{
    InputPin as InputPinHal, OutputPin as OutputPinHal, StatefulOutputPin as StatefulOutputPinHal,
    ToggleableOutputPin as ToggleableOutputPinHal,
};
use embedded_hal::pwm::{Pwm, PwmPin};

use super::{Error, InputPin, IoPin, Level, OutputPin, Pin};

impl InputPinHal for Pin {
    type Error = Infallible;

    fn try_is_high(&self) -> Result<bool, Self::Error> {
        Ok(Self::read(self) == Level::High)
    }

    fn try_is_low(&self) -> Result<bool, Self::Error> {
        Ok(Self::read(self) == Level::Low)
    }
}

impl InputPinHal for InputPin {
    type Error = Infallible;

    fn try_is_high(&self) -> Result<bool, Self::Error> {
        Ok(Self::is_high(self))
    }

    fn try_is_low(&self) -> Result<bool, Self::Error> {
        Ok(Self::is_low(self))
    }
}

impl InputPinHal for IoPin {
    type Error = Infallible;

    fn try_is_high(&self) -> Result<bool, Self::Error> {
        Ok(Self::is_high(self))
    }

    fn try_is_low(&self) -> Result<bool, Self::Error> {
        Ok(Self::is_low(self))
    }
}

impl InputPinHal for OutputPin {
    type Error = Infallible;

    fn try_is_high(&self) -> Result<bool, Self::Error> {
        Ok(Self::is_set_high(self))
    }

    fn try_is_low(&self) -> Result<bool, Self::Error> {
        Ok(Self::is_set_low(self))
    }
}

impl OutputPinHal for OutputPin {
    type Error = Infallible;

    fn try_set_low(&mut self) -> Result<(), Self::Error> {
        OutputPin::set_low(self);

        Ok(())
    }

    fn try_set_high(&mut self) -> Result<(), Self::Error> {
        OutputPin::set_high(self);

        Ok(())
    }
}

impl embedded_hal_0::digital::v2::OutputPin for OutputPin {
    type Error = Infallible;

    fn set_low(&mut self) -> Result<(), Self::Error> {
        self.try_set_low()
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        self.try_set_high()
    }
}

impl StatefulOutputPinHal for OutputPin {
    fn try_is_set_high(&self) -> Result<bool, Self::Error> {
        Ok(OutputPin::is_set_high(self))
    }

    fn try_is_set_low(&self) -> Result<bool, Self::Error> {
        Ok(OutputPin::is_set_low(self))
    }
}

impl ToggleableOutputPinHal for OutputPin {
    type Error = Infallible;

    fn try_toggle(&mut self) -> Result<(), Self::Error> {
        OutputPin::toggle(self);

        Ok(())
    }
}

impl OutputPinHal for IoPin {
    type Error = Infallible;

    fn try_set_low(&mut self) -> Result<(), Self::Error> {
        IoPin::set_low(self);

        Ok(())
    }

    fn try_set_high(&mut self) -> Result<(), Self::Error> {
        IoPin::set_high(self);

        Ok(())
    }
}

impl embedded_hal_0::digital::v2::OutputPin for IoPin {
    type Error = Infallible;

    fn set_low(&mut self) -> Result<(), Self::Error> {
        self.try_set_low()
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        self.try_set_high()
    }
}

impl StatefulOutputPinHal for IoPin {
    fn try_is_set_high(&self) -> Result<bool, Self::Error> {
        Ok(IoPin::is_high(self))
    }

    fn try_is_set_low(&self) -> Result<bool, Self::Error> {
        Ok(IoPin::is_low(self))
    }
}

impl ToggleableOutputPinHal for IoPin {
    type Error = Infallible;

    fn try_toggle(&mut self) -> Result<(), Self::Error> {
        IoPin::toggle(self);

        Ok(())
    }
}

const NANOS_PER_SEC: f64 = 1_000_000_000.0;

impl embedded_hal::pwm::Pwm for OutputPin {
    type Duty = f64;
    type Channel = ();
    type Time = Duration;
    type Error = Error;

    /// Disables a PWM `channel`
    fn try_disable(&mut self, _channel: Self::Channel) -> Result<(), Self::Error> {
        self.clear_pwm()
    }

    /// Enables a PWM `channel`
    fn try_enable(&mut self, _channel: Self::Channel) -> Result<(), Self::Error> {
        self.set_pwm_frequency(self.frequency, self.duty_cycle)
    }

    /// Returns the current PWM period
    fn try_get_period(&self) -> Result<Self::Time, Self::Error> {
        Ok(Duration::from_nanos(if self.frequency == 0.0 {
            0
        } else {
            ((1.0 / self.frequency) * NANOS_PER_SEC) as u64
        }))
    }

    /// Returns the current duty cycle
    fn try_get_duty(&self, _channel: Self::Channel) -> Result<Self::Duty, Self::Error> {
        Ok(self.duty_cycle)
    }

    /// Returns the maximum duty cycle value
    fn try_get_max_duty(&self) -> Result<Self::Duty, Self::Error> {
        Ok(1.0)
    }

    /// Sets a new duty cycle
    fn try_set_duty(
        &mut self,
        _channel: Self::Channel,
        duty: Self::Duty,
    ) -> Result<(), Self::Error> {
        self.duty_cycle = duty.max(0.0).min(1.0);

        if self.soft_pwm.is_some() {
            self.set_pwm_frequency(self.frequency, self.duty_cycle)?;
        }

        Ok(())
    }

    /// Sets a new PWM period
    fn try_set_period<P>(&mut self, period: P) -> Result<(), Self::Error>
    where
        P: Into<Self::Time>,
    {
        let period = period.into();
        self.frequency =
            1.0 / (period.as_secs() as f64 + (f64::from(period.subsec_nanos()) / NANOS_PER_SEC));

        if self.soft_pwm.is_some() {
            self.set_pwm_frequency(self.frequency, self.duty_cycle)?;
        }

        Ok(())
    }
}

impl PwmPin for OutputPin {
    type Duty = f64;
    type Error = Error;

    fn try_disable(&mut self) -> Result<(), Self::Error> {
        self.clear_pwm()
    }

    fn try_enable(&mut self) -> Result<(), Self::Error> {
        self.set_pwm_frequency(self.frequency, self.duty_cycle)
    }

    fn try_get_duty(&self) -> Result<Self::Duty, Self::Error> {
        Ok(self.duty_cycle)
    }

    fn try_get_max_duty(&self) -> Result<Self::Duty, Self::Error> {
        Ok(1.0)
    }

    fn try_set_duty(&mut self, duty: Self::Duty) -> Result<(), Self::Error> {
        self.duty_cycle = duty.max(0.0).min(1.0);

        if self.soft_pwm.is_some() {
            self.set_pwm_frequency(self.frequency, self.duty_cycle)?;
        }

        Ok(())
    }
}

impl embedded_hal_0::PwmPin for OutputPin {
    type Duty = f64;

    fn disable(&mut self) {
        let _ = PwmPin::try_disable(self);
    }

    fn enable(&mut self) {
        let _ = PwmPin::try_enable(self);
    }

    fn get_duty(&self) -> Self::Duty {
        PwmPin::try_get_duty(self).unwrap_or_default()
    }

    fn get_max_duty(&self) -> Self::Duty {
        PwmPin::try_get_max_duty(self).unwrap_or(1.0)
    }

    fn set_duty(&mut self, duty: Self::Duty) {
        let _ = PwmPin::try_set_duty(self, duty);
    }
}

impl Pwm for IoPin {
    type Duty = f64;
    type Channel = ();
    type Time = Duration;
    type Error = Error;

    /// Disables a PWM `channel`
    fn try_disable(&mut self, _channel: Self::Channel) -> Result<(), Self::Error> {
        self.clear_pwm()
    }

    /// Enables a PWM `channel`
    fn try_enable(&mut self, _channel: Self::Channel) -> Result<(), Self::Error> {
        self.set_pwm_frequency(self.frequency, self.duty_cycle)
    }

    /// Returns the current PWM period
    fn try_get_period(&self) -> Result<Self::Time, Self::Error> {
        Ok(Duration::from_nanos(if self.frequency == 0.0 {
            0
        } else {
            ((1.0 / self.frequency) * NANOS_PER_SEC) as u64
        }))
    }

    /// Returns the current duty cycle
    fn try_get_duty(&self, _channel: Self::Channel) -> Result<Self::Duty, Self::Error> {
        Ok(self.duty_cycle)
    }

    /// Returns the maximum duty cycle value
    fn try_get_max_duty(&self) -> Result<Self::Duty, Self::Error> {
        Ok(1.0)
    }

    /// Sets a new duty cycle
    fn try_set_duty(
        &mut self,
        _channel: Self::Channel,
        duty: Self::Duty,
    ) -> Result<(), Self::Error> {
        self.duty_cycle = duty.max(0.0).min(1.0);

        if self.soft_pwm.is_some() {
            self.set_pwm_frequency(self.frequency, self.duty_cycle)?;
        }

        Ok(())
    }

    /// Sets a new PWM period
    fn try_set_period<P>(&mut self, period: P) -> Result<(), Self::Error>
    where
        P: Into<Self::Time>,
    {
        let period = period.into();
        self.frequency =
            1.0 / (period.as_secs() as f64 + (f64::from(period.subsec_nanos()) / NANOS_PER_SEC));

        if self.soft_pwm.is_some() {
            self.set_pwm_frequency(self.frequency, self.duty_cycle)?;
        }

        Ok(())
    }
}

impl PwmPin for IoPin {
    type Duty = f64;
    type Error = Error;

    fn try_disable(&mut self) -> Result<(), Self::Error> {
        self.clear_pwm()
    }

    fn try_enable(&mut self) -> Result<(), Self::Error> {
        self.set_pwm_frequency(self.frequency, self.duty_cycle)
    }

    fn try_get_duty(&self) -> Result<Self::Duty, Self::Error> {
        Ok(self.duty_cycle)
    }

    fn try_get_max_duty(&self) -> Result<Self::Duty, Self::Error> {
        Ok(1.0)
    }

    fn try_set_duty(&mut self, duty: Self::Duty) -> Result<(), Self::Error> {
        self.duty_cycle = duty.max(0.0).min(1.0);

        if self.soft_pwm.is_some() {
            self.set_pwm_frequency(self.frequency, self.duty_cycle)?;
        }

        Ok(())
    }
}

impl embedded_hal_0::PwmPin for IoPin {
    type Duty = f64;

    fn disable(&mut self) {
        let _ = PwmPin::try_disable(self);
    }

    fn enable(&mut self) {
        let _ = PwmPin::try_enable(self);
    }

    fn get_duty(&self) -> Self::Duty {
        PwmPin::try_get_duty(self).unwrap_or_default()
    }

    fn get_max_duty(&self) -> Self::Duty {
        PwmPin::try_get_max_duty(self).unwrap_or(1.0)
    }

    fn set_duty(&mut self, duty: Self::Duty) {
        let _ = PwmPin::try_set_duty(self, duty);
    }
}

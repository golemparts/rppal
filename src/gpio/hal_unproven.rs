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

use std::time::Duration;
use core::convert::Infallible;

use embedded_hal::digital::{
    InputPin as InputPinHal,
    StatefulOutputPin as StatefulOutputPinHal,
    ToggleableOutputPin as ToggleableOutputPinHal,
};
use embedded_hal::pwm::Pwm;

use super::{InputPin, IoPin, Level, OutputPin, Pin, Error};

const NANOS_PER_SEC: f64 = 1_000_000_000.0;

impl InputPinHal for Pin {
    type Error = Infallible;

    fn try_is_high(&self) -> Result<bool, Self::Error> {
        Ok(Pin::read(self) == Level::High)
    }

    fn try_is_low(&self) -> Result<bool, Self::Error> {
        Ok(Pin::read(self) == Level::Low)
    }
}

impl embedded_hal_0::digital::v2::InputPin for Pin {
    type Error = Infallible;

    fn is_high(&self) -> Result<bool, Self::Error> {
        self.try_is_high()
    }

    fn is_low(&self) -> Result<bool, Self::Error> {
        self.try_is_low()
    }
}

impl InputPinHal for InputPin {
    type Error = Infallible;

    fn try_is_high(&self) -> Result<bool, Self::Error> {
        Ok(InputPin::is_high(self))
    }

    fn try_is_low(&self) -> Result<bool, Self::Error> {
        Ok(InputPin::is_low(self))
    }
}

impl embedded_hal_0::digital::v2::InputPin for InputPin {
    type Error = Infallible;

    fn is_high(&self) -> Result<bool, Self::Error> {
        self.try_is_high()
    }

    fn is_low(&self) -> Result<bool, Self::Error> {
        self.try_is_low()
    }
}

impl InputPinHal for IoPin {
    type Error = Infallible;

    fn try_is_high(&self) -> Result<bool, Self::Error> {
        Ok(IoPin::is_high(self))
    }

    fn try_is_low(&self) -> Result<bool, Self::Error> {
        Ok(IoPin::is_low(self))
    }
}

impl embedded_hal_0::digital::v2::InputPin for IoPin {
    type Error = Infallible;

    fn is_high(&self) -> Result<bool, Self::Error> {
        self.try_is_high()
    }

    fn is_low(&self) -> Result<bool, Self::Error> {
        self.try_is_low()
    }
}

impl InputPinHal for OutputPin {
    type Error = Infallible;

    fn try_is_high(&self) -> Result<bool, Self::Error> {
        Ok(OutputPin::is_set_high(self))
    }

    fn try_is_low(&self) -> Result<bool, Self::Error> {
        Ok(OutputPin::is_set_low(self))
    }
}

impl embedded_hal_0::digital::v2::InputPin for OutputPin {
    type Error = Infallible;

    fn is_high(&self) -> Result<bool, Self::Error> {
        self.try_is_high()
    }

    fn is_low(&self) -> Result<bool, Self::Error> {
        self.try_is_low()
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

impl embedded_hal_0::digital::v2::StatefulOutputPin for IoPin {
    fn is_set_high(&self) -> Result<bool, Self::Error> {
        self.try_is_set_high()
    }

    fn is_set_low(&self) -> Result<bool, Self::Error> {
        self.try_is_set_low()
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

impl embedded_hal_0::digital::v2::StatefulOutputPin for OutputPin {
    fn is_set_high(&self) -> Result<bool, Self::Error> {
        self.try_is_set_high()
    }

    fn is_set_low(&self) -> Result<bool, Self::Error> {
        self.try_is_set_low()
    }
}

impl ToggleableOutputPinHal for IoPin {
    type Error = Infallible;

    fn try_toggle(&mut self) -> Result<(), Self::Error> {
        IoPin::toggle(self);

        Ok(())
    }
}

impl embedded_hal_0::digital::v2::ToggleableOutputPin for IoPin {
    type Error = Infallible;

    fn toggle(&mut self) -> Result<(), Self::Error> {
        self.try_toggle()
    }
}

impl ToggleableOutputPinHal for OutputPin {
    type Error = Infallible;

    fn try_toggle(&mut self) -> Result<(), Self::Error> {
        OutputPin::toggle(self);

        Ok(())
    }
}

impl embedded_hal_0::digital::v2::ToggleableOutputPin for OutputPin {
    type Error = Infallible;

    fn toggle(&mut self) -> Result<(), Self::Error> {
        self.try_toggle()
    }
}

impl Pwm for OutputPin {
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
    fn try_set_duty(&mut self, _channel: Self::Channel, duty: Self::Duty) -> Result<(), Self::Error> {
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

impl embedded_hal_0::Pwm for OutputPin {
    type Duty = f64;
    type Channel = ();
    type Time = Duration;

    /// Disables a PWM `channel`
    fn disable(&mut self, channel: Self::Channel) {
        let _ = self.try_disable(channel);
    }

    /// Enables a PWM `channel`
    fn enable(&mut self, channel: Self::Channel) {
        let _ = self.try_enable(channel);
    }

    /// Returns the current PWM period
    fn get_period(&self) -> Self::Time {
        self.try_get_period().unwrap_or_default()
    }

    /// Returns the current duty cycle
    fn get_duty(&self, channel: Self::Channel) -> Self::Duty {
        self.try_get_duty(channel).unwrap_or_default()
    }

    /// Returns the maximum duty cycle value
    fn get_max_duty(&self) -> Self::Duty {
        self.try_get_max_duty().unwrap_or(1.0)
    }

    /// Sets a new duty cycle
    fn set_duty(&mut self, channel: Self::Channel, duty: Self::Duty) {
        let _ = self.try_set_duty(channel, duty);
    }

    /// Sets a new PWM period
    fn set_period<P>(&mut self, period: P)
    where
        P: Into<Self::Time>,
    {
        let _ = self.try_set_period(period);
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
    fn try_set_duty(&mut self, _channel: Self::Channel, duty: Self::Duty) -> Result<(), Self::Error> {
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

impl embedded_hal_0::Pwm for IoPin {
    type Duty = f64;
    type Channel = ();
    type Time = Duration;

    /// Disables a PWM `channel`
    fn disable(&mut self, channel: Self::Channel) {
        let _ = self.try_disable(channel);
    }

    /// Enables a PWM `channel`
    fn enable(&mut self, channel: Self::Channel) {
        let _ = self.try_enable(channel);
    }

    /// Returns the current PWM period
    fn get_period(&self) -> Self::Time {
        self.try_get_period().unwrap_or_default()
    }

    /// Returns the current duty cycle
    fn get_duty(&self, channel: Self::Channel) -> Self::Duty {
        self.try_get_duty(channel).unwrap_or_default()
    }

    /// Returns the maximum duty cycle value
    fn get_max_duty(&self) -> Self::Duty {
        self.try_get_max_duty().unwrap_or(1.0)
    }

    /// Sets a new duty cycle
    fn set_duty(&mut self, channel: Self::Channel, duty: Self::Duty) {
        let _ = self.try_set_duty(channel, duty);
    }

    /// Sets a new PWM period
    fn set_period<P>(&mut self, period: P)
    where
        P: Into<Self::Time>,
    {
        let _ = self.try_set_period(period);
    }
}

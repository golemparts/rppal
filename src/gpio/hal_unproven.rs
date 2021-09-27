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

use embedded_hal::digital::blocking::{
    InputPin as InputPinHal, StatefulOutputPin as StatefulOutputPinHal,
    ToggleableOutputPin as ToggleableOutputPinHal,
};
use embedded_hal::pwm::blocking::Pwm as PwmHal;

use super::{InputPin, IoPin, OutputPin, Pin};

impl embedded_hal_0::digital::v2::InputPin for Pin {
    type Error = Infallible;

    fn is_high(&self) -> Result<bool, Self::Error> {
        InputPinHal::is_high(self)
    }

    fn is_low(&self) -> Result<bool, Self::Error> {
        InputPinHal::is_low(self)
    }
}

impl embedded_hal_0::digital::v2::InputPin for InputPin {
    type Error = Infallible;

    fn is_high(&self) -> Result<bool, Self::Error> {
        InputPinHal::is_high(self)
    }

    fn is_low(&self) -> Result<bool, Self::Error> {
        InputPinHal::is_low(self)
    }
}

impl embedded_hal_0::digital::v2::InputPin for IoPin {
    type Error = Infallible;

    fn is_high(&self) -> Result<bool, Self::Error> {
        InputPinHal::is_high(self)
    }

    fn is_low(&self) -> Result<bool, Self::Error> {
        InputPinHal::is_low(self)
    }
}

impl embedded_hal_0::digital::v2::InputPin for OutputPin {
    type Error = Infallible;

    fn is_high(&self) -> Result<bool, Self::Error> {
        InputPinHal::is_high(self)
    }

    fn is_low(&self) -> Result<bool, Self::Error> {
        InputPinHal::is_low(self)
    }
}

impl embedded_hal_0::digital::v2::StatefulOutputPin for IoPin {
    fn is_set_high(&self) -> Result<bool, Self::Error> {
        StatefulOutputPinHal::is_set_high(self)
    }

    fn is_set_low(&self) -> Result<bool, Self::Error> {
        StatefulOutputPinHal::is_set_low(self)
    }
}

impl embedded_hal_0::digital::v2::StatefulOutputPin for OutputPin {
    fn is_set_high(&self) -> Result<bool, Self::Error> {
        StatefulOutputPinHal::is_set_high(self)
    }

    fn is_set_low(&self) -> Result<bool, Self::Error> {
        StatefulOutputPinHal::is_set_low(self)
    }
}

impl embedded_hal_0::digital::v2::ToggleableOutputPin for IoPin {
    type Error = Infallible;

    fn toggle(&mut self) -> Result<(), Self::Error> {
        ToggleableOutputPinHal::toggle(self)
    }
}

impl embedded_hal_0::digital::v2::ToggleableOutputPin for OutputPin {
    type Error = Infallible;

    fn toggle(&mut self) -> Result<(), Self::Error> {
        ToggleableOutputPinHal::toggle(self)
    }
}

impl embedded_hal_0::Pwm for OutputPin {
    type Duty = f64;
    type Channel = ();
    type Time = Duration;

    /// Disables a PWM `channel`
    fn disable(&mut self, channel: Self::Channel) {
        let _ = PwmHal::disable(self, &channel);
    }

    /// Enables a PWM `channel`
    fn enable(&mut self, channel: Self::Channel) {
        let _ = PwmHal::enable(self, &channel);
    }

    /// Returns the current PWM period
    fn get_period(&self) -> Self::Time {
        PwmHal::get_period(self).unwrap_or_default()
    }

    /// Returns the current duty cycle
    fn get_duty(&self, channel: Self::Channel) -> Self::Duty {
        PwmHal::get_duty(self, &channel).unwrap_or_default()
    }

    /// Returns the maximum duty cycle value
    fn get_max_duty(&self) -> Self::Duty {
        PwmHal::get_max_duty(self).unwrap_or(1.0)
    }

    /// Sets a new duty cycle
    fn set_duty(&mut self, channel: Self::Channel, duty: Self::Duty) {
        let _ = PwmHal::set_duty(self, &channel, duty);
    }

    /// Sets a new PWM period
    fn set_period<P>(&mut self, period: P)
    where
        P: Into<Self::Time>,
    {
        let _ = PwmHal::set_period(self, period);
    }
}

impl embedded_hal_0::Pwm for IoPin {
    type Duty = f64;
    type Channel = ();
    type Time = Duration;

    /// Disables a PWM `channel`
    fn disable(&mut self, channel: Self::Channel) {
        let _ = PwmHal::disable(self, &channel);
    }

    /// Enables a PWM `channel`
    fn enable(&mut self, channel: Self::Channel) {
        let _ = PwmHal::enable(self, &channel);
    }

    /// Returns the current PWM period
    fn get_period(&self) -> Self::Time {
        PwmHal::get_period(self).unwrap_or_default()
    }

    /// Returns the current duty cycle
    fn get_duty(&self, channel: Self::Channel) -> Self::Duty {
        PwmHal::get_duty(self, &channel).unwrap_or_default()
    }

    /// Returns the maximum duty cycle value
    fn get_max_duty(&self) -> Self::Duty {
        PwmHal::get_max_duty(self).unwrap_or(1.0)
    }

    /// Sets a new duty cycle
    fn set_duty(&mut self, channel: Self::Channel, duty: Self::Duty) {
        let _ = PwmHal::set_duty(self, &channel, duty);
    }

    /// Sets a new PWM period
    fn set_period<P>(&mut self, period: P)
    where
        P: Into<Self::Time>,
    {
        let _ = PwmHal::set_period(self, period);
    }
}

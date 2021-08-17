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

use embedded_hal::pwm::Pwm as PwmHal;

use super::{Pwm, Error};

impl PwmHal for Pwm {
    type Duty = f64;
    type Channel = ();
    type Time = Duration;
    type Error = Error;

    /// Disables a PWM `channel`
    fn try_disable(&mut self, _channel: Self::Channel) -> Result<(), Self::Error> {
        Pwm::disable(self)
    }

    /// Enables a PWM `channel`
    fn try_enable(&mut self, _channel: Self::Channel) -> Result<(), Self::Error> {
        Pwm::enable(self)
    }

    /// Returns the current PWM period
    fn try_get_period(&self) -> Result<Self::Time, Self::Error> {
        self.period()
    }

    /// Returns the current duty cycle
    fn try_get_duty(&self, _channel: Self::Channel) -> Result<Self::Duty, Self::Error> {
        self.duty_cycle()
    }

    /// Returns the maximum duty cycle value
    fn try_get_max_duty(&self) -> Result<Self::Duty, Self::Error> {
        Ok(1.0)
    }

    /// Sets a new duty cycle
    fn try_set_duty(&mut self, _channel: Self::Channel, duty: Self::Duty) -> Result<(), Self::Error> {
        self.set_duty_cycle(duty)
    }

    /// Sets a new PWM period
    fn try_set_period<P>(&mut self, period: P) -> Result<(), Self::Error>
    where
        P: Into<Self::Time>,
    {
        Pwm::set_period(self, period.into())
    }
}

impl embedded_hal_0::Pwm for Pwm {
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

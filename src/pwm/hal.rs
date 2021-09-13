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

use embedded_hal::pwm::blocking::{Pwm as PwmHal, PwmPin as PwmPinHal};

use super::{Error, Pwm};

impl PwmHal for Pwm {
    type Duty = f64;
    type Channel = ();
    type Time = Duration;
    type Error = Error;

    /// Disables a PWM `channel`
    fn disable(&mut self, _channel: &Self::Channel) -> Result<(), Self::Error> {
        Pwm::disable(self)
    }

    /// Enables a PWM `channel`
    fn enable(&mut self, _channel: &Self::Channel) -> Result<(), Self::Error> {
        Pwm::enable(self)
    }

    /// Returns the current PWM period
    fn get_period(&self) -> Result<Self::Time, Self::Error> {
        self.period()
    }

    /// Returns the current duty cycle
    fn get_duty(&self, _channel: &Self::Channel) -> Result<Self::Duty, Self::Error> {
        self.duty_cycle()
    }

    /// Returns the maximum duty cycle value
    fn get_max_duty(&self) -> Result<Self::Duty, Self::Error> {
        Ok(1.0)
    }

    /// Sets a new duty cycle
    fn set_duty(
        &mut self,
        _channel: &Self::Channel,
        duty: Self::Duty,
    ) -> Result<(), Self::Error> {
        self.set_duty_cycle(duty)
    }

    /// Sets a new PWM period
    fn set_period<P>(&mut self, period: P) -> Result<(), Self::Error>
    where
        P: Into<Self::Time>,
    {
        Pwm::set_period(self, period.into())
    }
}

impl PwmPinHal for Pwm {
    type Duty = f64;
    type Error = Error;

    fn disable(&mut self) -> Result<(), Self::Error> {
        Pwm::disable(self)
    }

    fn enable(&mut self) -> Result<(), Self::Error> {
        Pwm::enable(self)
    }

    fn get_duty(&self) -> Result<Self::Duty, Self::Error> {
        self.duty_cycle()
    }

    fn get_max_duty(&self) -> Result<Self::Duty, Self::Error> {
        Ok(1.0)
    }

    fn set_duty(&mut self, duty: Self::Duty) -> Result<(), Self::Error> {
        self.set_duty_cycle(duty)
    }
}

impl embedded_hal_0::PwmPin for Pwm {
    type Duty = f64;

    fn disable(&mut self) {
        let _ = PwmPinHal::disable(self);
    }

    fn enable(&mut self) {
        let _ = PwmPinHal::enable(self);
    }

    fn get_duty(&self) -> Self::Duty {
        PwmPinHal::get_duty(self).unwrap_or_default()
    }

    fn get_max_duty(&self) -> Self::Duty {
        PwmPinHal::get_max_duty(self).unwrap()
    }

    fn set_duty(&mut self, duty: Self::Duty) {
        let _ = PwmPinHal::set_duty(self, duty);
    }
}

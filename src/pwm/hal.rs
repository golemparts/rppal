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

use embedded_hal::pwm::PwmPin;

use super::{Pwm, Error};

impl PwmPin for Pwm {
    type Duty = f64;
    type Error = Error;

    fn try_disable(&mut self) -> Result<(), Self::Error> {
        Pwm::disable(self)
    }

    fn try_enable(&mut self) -> Result<(), Self::Error> {
        Pwm::enable(self)
    }

    fn try_get_duty(&self) -> Result<Self::Duty, Self::Error> {
        self.duty_cycle()
    }

    fn try_get_max_duty(&self) -> Result<Self::Duty, Self::Error> {
        Ok(1.0)
    }

    fn try_set_duty(&mut self, duty: Self::Duty) -> Result<(), Self::Error> {
        self.set_duty_cycle(duty)
    }
}

impl embedded_hal_0::PwmPin for Pwm {
    type Duty = f64;

    fn disable(&mut self) {
        let _ = self.try_disable();
    }

    fn enable(&mut self) {
        let _ = self.try_enable();
    }

    fn get_duty(&self) -> Self::Duty {
        self.try_get_duty().unwrap_or_default()
    }

    fn get_max_duty(&self) -> Self::Duty {
        self.try_get_max_duty().unwrap()
    }

    fn set_duty(&mut self, duty: Self::Duty) {
        let _ = self.try_set_duty(duty);
    }
}

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

use embedded_hal::digital::OutputPin as OutputPinHal;
use embedded_hal::pwm::PwmPin;

use super::{Error, IoPin, OutputPin};

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
        let _ = self.try_disable();
    }

    fn enable(&mut self) {
        let _ = self.try_enable();
    }

    fn get_duty(&self) -> Self::Duty {
        self.try_get_duty().unwrap_or_default()
    }

    fn get_max_duty(&self) -> Self::Duty {
        self.try_get_max_duty().unwrap_or(1.0)
    }

    fn set_duty(&mut self, duty: Self::Duty) {
        let _ = self.try_set_duty(duty);
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
        let _ = self.try_disable();
    }

    fn enable(&mut self) {
        let _ = self.try_enable();
    }

    fn get_duty(&self) -> Self::Duty {
        self.try_get_duty().unwrap_or_default()
    }

    fn get_max_duty(&self) -> Self::Duty {
        self.try_get_max_duty().unwrap_or(1.0)
    }

    fn set_duty(&mut self, duty: Self::Duty) {
        let _ = self.try_set_duty(duty);
    }
}

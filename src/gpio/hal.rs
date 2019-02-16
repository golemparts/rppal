// Copyright (c) 2017-2019 Rene van der Meer
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

use embedded_hal::digital;
use embedded_hal::PwmPin;

use super::{IoPin, OutputPin};

impl digital::OutputPin for OutputPin {
    fn set_low(&mut self) {
        OutputPin::set_low(self);
    }

    fn set_high(&mut self) {
        OutputPin::set_high(self);
    }
}

impl digital::OutputPin for IoPin {
    fn set_low(&mut self) {
        IoPin::set_low(self);
    }

    fn set_high(&mut self) {
        IoPin::set_high(self);
    }
}

impl PwmPin for OutputPin {
    type Duty = f64;

    fn disable(&mut self) {
        let _ = self.clear_pwm();
    }

    fn enable(&mut self) {
        let _ = self.set_pwm_frequency(self.frequency, self.duty_cycle);
    }

    fn get_duty(&self) -> Self::Duty {
        self.duty_cycle
    }

    fn get_max_duty(&self) -> Self::Duty {
        1.0
    }

    fn set_duty(&mut self, duty: Self::Duty) {
        self.duty_cycle = duty.max(0.0).min(1.0);

        if self.soft_pwm.is_some() {
            let _ = self.set_pwm_frequency(self.frequency, self.duty_cycle);
        }
    }
}

impl PwmPin for IoPin {
    type Duty = f64;

    fn disable(&mut self) {
        let _ = self.clear_pwm();
    }

    fn enable(&mut self) {
        let _ = self.set_pwm_frequency(self.frequency, self.duty_cycle);
    }

    fn get_duty(&self) -> Self::Duty {
        self.duty_cycle
    }

    fn get_max_duty(&self) -> Self::Duty {
        1.0
    }

    fn set_duty(&mut self, duty: Self::Duty) {
        self.duty_cycle = duty.max(0.0).min(1.0);

        if self.soft_pwm.is_some() {
            let _ = self.set_pwm_frequency(self.frequency, self.duty_cycle);
        }
    }
}

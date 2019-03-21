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

use std::time::Duration;

use embedded_hal::digital;
use embedded_hal::Pwm;

use super::{InputPin, IoPin, Level, OutputPin, Pin};

impl digital::InputPin for Pin {
    fn is_high(&self) -> bool {
        Pin::read(self) == Level::High
    }

    fn is_low(&self) -> bool {
        Pin::read(self) == Level::Low
    }
}

impl digital::InputPin for InputPin {
    fn is_high(&self) -> bool {
        InputPin::is_high(self)
    }

    fn is_low(&self) -> bool {
        InputPin::is_low(self)
    }
}

impl digital::InputPin for IoPin {
    fn is_high(&self) -> bool {
        IoPin::is_high(self)
    }

    fn is_low(&self) -> bool {
        IoPin::is_low(self)
    }
}

impl digital::InputPin for OutputPin {
    fn is_high(&self) -> bool {
        OutputPin::is_set_high(self)
    }

    fn is_low(&self) -> bool {
        OutputPin::is_set_low(self)
    }
}

impl digital::StatefulOutputPin for IoPin {
    fn is_set_high(&self) -> bool {
        IoPin::is_high(self)
    }

    fn is_set_low(&self) -> bool {
        IoPin::is_low(self)
    }
}

impl digital::StatefulOutputPin for OutputPin {
    fn is_set_high(&self) -> bool {
        OutputPin::is_set_high(self)
    }

    fn is_set_low(&self) -> bool {
        OutputPin::is_set_low(self)
    }
}

impl digital::ToggleableOutputPin for IoPin {
    fn toggle(&mut self) {
        IoPin::toggle(self);
    }
}

impl digital::ToggleableOutputPin for OutputPin {
    fn toggle(&mut self) {
        OutputPin::toggle(self);
    }
}

impl Pwm for OutputPin {
    type Duty = f64;
    type Channel = ();
    type Time = Duration;

    /// Disables a PWM `channel`
    fn disable(&mut self, _channel: Self::Channel) {
        let _ = self.clear_pwm();
    }

    /// Enables a PWM `channel`
    fn enable(&mut self, _channel: Self::Channel) {
        let _ = self.set_pwm_frequency(self.frequency, self.duty_cycle);
    }

    /// Returns the current PWM period
    fn get_period(&self) -> Self::Time {
        Duration::from_nanos(if self.frequency == 0.0 {
            0
        } else {
            ((1.0 / self.frequency) * 1_000_000_000.0) as u64
        })
    }

    /// Returns the current duty cycle
    fn get_duty(&self, _channel: Self::Channel) -> Self::Duty {
        self.duty_cycle
    }

    /// Returns the maximum duty cycle value
    fn get_max_duty(&self) -> Self::Duty {
        1.0
    }

    /// Sets a new duty cycle
    fn set_duty(&mut self, _channel: Self::Channel, duty: Self::Duty) {
        self.duty_cycle = duty.max(0.0).min(1.0);

        if self.soft_pwm.is_some() {
            let _ = self.set_pwm_frequency(self.frequency, self.duty_cycle);
        }
    }

    /// Sets a new PWM period
    fn set_period<P>(&mut self, period: P)
    where
        P: Into<Self::Time>,
    {
        let period = period.into();
        self.frequency =
            1.0 / (period.as_secs() as f64 + (f64::from(period.subsec_nanos()) / 1_000_000_000.0));

        if self.soft_pwm.is_some() {
            let _ = self.set_pwm_frequency(self.frequency, self.duty_cycle);
        }
    }
}

impl Pwm for IoPin {
    type Duty = f64;
    type Channel = ();
    type Time = Duration;

    /// Disables a PWM `channel`
    fn disable(&mut self, _channel: Self::Channel) {
        let _ = self.clear_pwm();
    }

    /// Enables a PWM `channel`
    fn enable(&mut self, _channel: Self::Channel) {
        let _ = self.set_pwm_frequency(self.frequency, self.duty_cycle);
    }

    /// Returns the current PWM period
    fn get_period(&self) -> Self::Time {
        Duration::from_nanos(if self.frequency == 0.0 {
            0
        } else {
            ((1.0 / self.frequency) * 1_000_000_000.0) as u64
        })
    }

    /// Returns the current duty cycle
    fn get_duty(&self, _channel: Self::Channel) -> Self::Duty {
        self.duty_cycle
    }

    /// Returns the maximum duty cycle value
    fn get_max_duty(&self) -> Self::Duty {
        1.0
    }

    /// Sets a new duty cycle
    fn set_duty(&mut self, _channel: Self::Channel, duty: Self::Duty) {
        self.duty_cycle = duty.max(0.0).min(1.0);

        if self.soft_pwm.is_some() {
            let _ = self.set_pwm_frequency(self.frequency, self.duty_cycle);
        }
    }

    /// Sets a new PWM period
    fn set_period<P>(&mut self, period: P)
    where
        P: Into<Self::Time>,
    {
        let period = period.into();
        self.frequency =
            1.0 / (period.as_secs() as f64 + (f64::from(period.subsec_nanos()) / 1_000_000_000.0));

        if self.soft_pwm.is_some() {
            let _ = self.set_pwm_frequency(self.frequency, self.duty_cycle);
        }
    }
}

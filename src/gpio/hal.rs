use core::convert::Infallible;

use embedded_hal::digital::{
    ErrorType, InputPin as InputPinHal, OutputPin as OutputPinHal,
    StatefulOutputPin as StatefulOutputPinHal, ToggleableOutputPin as ToggleableOutputPinHal,
};

use super::{InputPin, IoPin, Level, OutputPin, Pin};

/// `ErrorType` trait implementation for `embedded-hal` v1.0.0-alpha.7.
impl ErrorType for Pin {
    type Error = Infallible;
}

/// `InputPin` trait implementation for `embedded-hal` v1.0.0-alpha.7.
impl InputPinHal for Pin {
    fn is_high(&self) -> Result<bool, Self::Error> {
        Ok(Self::read(self) == Level::High)
    }

    fn is_low(&self) -> Result<bool, Self::Error> {
        Ok(Self::read(self) == Level::Low)
    }
}

/// `ErrorType` trait implementation for `embedded-hal` v1.0.0-alpha.7.
impl ErrorType for InputPin {
    type Error = Infallible;
}

/// `InputPin` trait implementation for `embedded-hal` v1.0.0-alpha.7.
impl InputPinHal for InputPin {
    fn is_high(&self) -> Result<bool, Self::Error> {
        Ok(Self::is_high(self))
    }

    fn is_low(&self) -> Result<bool, Self::Error> {
        Ok(Self::is_low(self))
    }
}

/// `ErrorType` trait implementation for `embedded-hal` v1.0.0-alpha.7.
impl ErrorType for IoPin {
    type Error = Infallible;
}

/// `InputPin` trait implementation for `embedded-hal` v1.0.0-alpha.7.
impl InputPinHal for IoPin {
    fn is_high(&self) -> Result<bool, Self::Error> {
        Ok(Self::is_high(self))
    }

    fn is_low(&self) -> Result<bool, Self::Error> {
        Ok(Self::is_low(self))
    }
}

/// `ErrorType` trait implementation for `embedded-hal` v1.0.0-alpha.7.
impl ErrorType for OutputPin {
    type Error = Infallible;
}

/// `InputPin` trait implementation for `embedded-hal` v1.0.0-alpha.7.
impl InputPinHal for OutputPin {
    fn is_high(&self) -> Result<bool, Self::Error> {
        Ok(Self::is_set_high(self))
    }

    fn is_low(&self) -> Result<bool, Self::Error> {
        Ok(Self::is_set_low(self))
    }
}

/// `OutputPin` trait implementation for `embedded-hal` v1.0.0-alpha.7.
impl OutputPinHal for OutputPin {
    fn set_low(&mut self) -> Result<(), Self::Error> {
        OutputPin::set_low(self);

        Ok(())
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        OutputPin::set_high(self);

        Ok(())
    }
}

/// `OutputPin` trait implementation for `embedded-hal` v0.2.6.
impl embedded_hal_0::digital::v2::OutputPin for OutputPin {
    type Error = Infallible;

    fn set_low(&mut self) -> Result<(), Self::Error> {
        OutputPinHal::set_low(self)
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        OutputPinHal::set_high(self)
    }
}

/// `StatefulOutputPin` trait implementation for `embedded-hal` v1.0.0-alpha.7.
impl StatefulOutputPinHal for OutputPin {
    fn is_set_high(&self) -> Result<bool, Self::Error> {
        Ok(OutputPin::is_set_high(self))
    }

    fn is_set_low(&self) -> Result<bool, Self::Error> {
        Ok(OutputPin::is_set_low(self))
    }
}

/// `ToggleableOutputPin` trait implementation for `embedded-hal` v1.0.0-alpha.7.
impl ToggleableOutputPinHal for OutputPin {
    fn toggle(&mut self) -> Result<(), Self::Error> {
        OutputPin::toggle(self);

        Ok(())
    }
}

/// `OutputPin` trait implementation for `embedded-hal` v1.0.0-alpha.7.
impl OutputPinHal for IoPin {
    fn set_low(&mut self) -> Result<(), Self::Error> {
        IoPin::set_low(self);

        Ok(())
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        IoPin::set_high(self);

        Ok(())
    }
}

/// `OutputPin` trait implementation for `embedded-hal` v0.2.6.
impl embedded_hal_0::digital::v2::OutputPin for IoPin {
    type Error = Infallible;

    fn set_low(&mut self) -> Result<(), Self::Error> {
        OutputPinHal::set_low(self)
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        OutputPinHal::set_high(self)
    }
}

/// `StatefulOutputPin` trait implementation for `embedded-hal` v1.0.0-alpha.7.
impl StatefulOutputPinHal for IoPin {
    fn is_set_high(&self) -> Result<bool, Self::Error> {
        Ok(IoPin::is_high(self))
    }

    fn is_set_low(&self) -> Result<bool, Self::Error> {
        Ok(IoPin::is_low(self))
    }
}

/// `ToggleableOutputPin` trait implementation for `embedded-hal` v1.0.0-alpha.7.
impl ToggleableOutputPinHal for IoPin {
    fn toggle(&mut self) -> Result<(), Self::Error> {
        IoPin::toggle(self);

        Ok(())
    }
}

/// `PwmPin` trait implementation for `embedded-hal` v0.2.6.
impl embedded_hal_0::PwmPin for OutputPin {
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

/// `PwmPin` trait implementation for `embedded-hal` v0.2.6.
impl embedded_hal_0::PwmPin for IoPin {
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

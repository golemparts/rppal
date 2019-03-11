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

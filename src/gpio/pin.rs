// Copyright (c) 2017-2018 Rene van der Meer
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

use std::os::unix::io::AsRawFd;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use crate::gpio::{
    interrupt::AsyncInterrupt,
    GpioState, Level, Mode,
    PullUpDown::{self, *},
    Result, Trigger,
};

// Maximum GPIO pins on the BCM2835. The actual number of pins
// exposed through the Pi's GPIO header depends on the model.
pub const MAX: usize = 54;

/// Unconfigured GPIO pin.
#[derive(Debug)]
pub struct Pin {
    pub(crate) pin: u8,
    gpio_state: Arc<GpioState>,
}

impl Pin {
    #[inline]
    pub(crate) fn new(pin: u8, gpio_state: Arc<GpioState>) -> Pin {
        Pin { pin, gpio_state }
    }

    /// Consumes the pin, returns an [`InputPin`] and sets its mode to [`Mode::Input`].
    ///
    /// [`InputPin`]: struct.InputPin.html
    /// [`Mode::Input`]: enum.Mode.html#variant.Input
    #[inline]
    pub fn into_input(self) -> InputPin {
        InputPin::new(self, Off)
    }

    /// Additionally to `into_input`, activates the pin's pull-up resistor.
    #[inline]
    pub fn into_input_pullup(self) -> InputPin {
        InputPin::new(self, PullUp)
    }

    /// Additionally to `into_input`, activates the pin's pull-down resistor.
    #[inline]
    pub fn into_input_pulldown(self) -> InputPin {
        InputPin::new(self, PullDown)
    }

    /// Consumes the pin, returns an [`OutputPin`] and sets its mode to [`Mode::Output`].
    ///
    /// [`OutputPin`]: struct.OutputPin.html
    /// [`Mode::Output`]: enum.Mode.html#variant.Output
    #[inline]
    pub fn into_output(self) -> OutputPin {
        OutputPin::new(self)
    }

    /// Consumes the pin, returns an [`AltPin`] and sets its mode to the given mode.
    ///
    /// [`AltPin`]: struct.AltPin.html
    /// [`Mode`]: enum.Mode.html
    #[inline]
    pub fn into_alt(self, mode: Mode) -> AltPin {
        AltPin::new(self, mode)
    }

    #[inline]
    pub(crate) fn set_mode(&mut self, mode: Mode) {
        self.gpio_state.gpio_mem.set_mode(self.pin, mode);
    }

    /// Returns the current GPIO pin mode.
    #[inline]
    pub fn mode(&self) -> Mode {
        self.gpio_state.gpio_mem.mode(self.pin)
    }

    /// Configures the built-in GPIO pull-up/pull-down resistors.
    #[inline]
    pub(crate) fn set_pullupdown(&self, pud: PullUpDown) {
        self.gpio_state.gpio_mem.set_pullupdown(self.pin, pud);
    }

    /// Reads the pin's current logic level.
    #[inline]
    pub fn read(&self) -> Level {
        self.gpio_state.gpio_mem.level(self.pin)
    }

    #[inline]
    pub(crate) fn set_low(&mut self) {
        self.gpio_state.gpio_mem.set_low(self.pin);
    }

    #[inline]
    pub(crate) fn set_high(&mut self) {
        self.gpio_state.gpio_mem.set_high(self.pin);
    }

    #[inline]
    pub(crate) fn write(&mut self, level: Level) {
        match level {
            Level::Low => self.set_low(),
            Level::High => self.set_high(),
        };
    }
}

impl Drop for Pin {
    fn drop(&mut self) {
        // Release taken pin
        self.gpio_state.pins_taken[self.pin as usize].store(false, Ordering::SeqCst);
    }
}

macro_rules! impl_input {
    () => {
        /// Reads the pin's current logic level.
        #[inline]
        pub fn read(&self) -> Level {
            self.pin.read()
        }

        /// Returns `true` if the pin's logic level is [`Level::Low`].
        ///
        /// [`Level::Low`]: enum.Level.html
        #[inline]
        pub fn is_low(&self) -> bool {
            self.pin.read() == Level::Low
        }

        /// Returns `true` if the pin's logic level is [`Level::High`].
        ///
        /// [`Level::High`]: enum.Level.html
        #[inline]
        pub fn is_high(&self) -> bool {
            self.pin.read() == Level::High
        }
    }
}

macro_rules! impl_output {
    () => {
        /// Sets pin's logic level to [`Level::Low`].
        ///
        /// [`Level::Low`]: enum.Level.html
        #[inline]
        pub fn set_low(&mut self) {
            self.pin.set_low()
        }

        /// Sets pin's logic level to [`Level::High`].
        ///
        /// [`Level::High`]: enum.Level.html
        #[inline]
        pub fn set_high(&mut self) {
            self.pin.set_high()
        }

        /// Sets pin's logic level.
        #[inline]
        pub fn write(&mut self, level: Level) {
            self.pin.write(level)
        }
    }
}

macro_rules! impl_clear_on_drop {
    () => {
        /// Returns the value of `clear_on_drop`.
        pub fn clear_on_drop(&self) -> bool {
            self.clear_on_drop
        }

        /// When enabled, resets pin's mode to its original state when it goes out of scope.
        /// By default, this is set to `true`.
        ///
        /// # Note
        ///
        /// Drop methods aren't called when a program is abnormally terminated, for
        /// instance when a user presses <kbd>Ctrl + C</kbd>, and the `SIGINT` signal
        /// isn't caught. You catch those using crates such as [`simple_signal`].
        ///
        /// [`simple_signal`]: https://crates.io/crates/simple-signal
        /// [`cleanup`]: #method.cleanup
        pub fn set_clear_on_drop(&mut self, clear_on_drop: bool) {
            self.clear_on_drop = clear_on_drop;
        }
    };
}

macro_rules! impl_drop {
    ($struct:ident) => {
        impl Drop for $struct {
            /// Resets the pin's mode if `clear_on_drop` is set to `true` (default).
            fn drop(&mut self) {
                if !self.clear_on_drop {
                    return;
                }

                if let Some(prev_mode) = self.prev_mode {
                    self.pin.set_mode(prev_mode)
                }
            }
        }
    };
}

/// GPIO pin configured as input.
#[derive(Debug)]
pub struct InputPin {
    pub(crate) pin: Pin,
    prev_mode: Option<Mode>,
    async_interrupt: Option<AsyncInterrupt>,
    clear_on_drop: bool,
}

impl InputPin {
    pub(crate) fn new(mut pin: Pin, pud_mode: PullUpDown) -> InputPin {
        let prev_mode = pin.mode();

        let prev_mode = if prev_mode == Mode::Input {
            None
        } else {
            pin.set_mode(Mode::Input);
            Some(prev_mode)
        };

        pin.set_pullupdown(pud_mode);

        InputPin {
            pin,
            prev_mode,
            async_interrupt: None,
            clear_on_drop: true,
        }
    }

    impl_input!();

    /// Configures a synchronous interrupt trigger.
    ///
    /// After configuring a synchronous interrupt trigger, use [`poll_interrupt`] or
    /// [`Gpio::poll_interrupts`] to block while waiting for a trigger event.
    ///
    /// Any previously configured (a)synchronous interrupt triggers will be cleared.
    ///
    /// [`poll_interrupt`]: #method.poll_interrupt
    /// [`Gpio::poll_interrupts`]: struct.Gpio#method.poll_interrupts
    pub fn set_interrupt(&mut self, trigger: Trigger) -> Result<()> {
        self.clear_async_interrupt()?;

        // Each pin can only be configured for a single trigger type
        (*self.pin.gpio_state.sync_interrupts.lock().unwrap()).set_interrupt(self.pin.pin, trigger)
    }

    /// Removes a previously configured synchronous interrupt trigger.
    pub fn clear_interrupt(&mut self) -> Result<()> {
        (*self.pin.gpio_state.sync_interrupts.lock().unwrap()).clear_interrupt(self.pin.pin)
    }

    /// Blocks until an interrupt is triggered on the pin, or a timeout occurs.
    ///
    /// This only works after the pin has been configured for synchronous interrupts using
    /// [`set_interrupt`]. Asynchronous interrupt triggers are automatically polled on a separate thread.
    ///
    /// Calling `poll_interrupt` blocks any other calls to `poll_interrupt` (including on other `InputPin`s) or
    /// [`Gpio::poll_interrupts`] until it returns. If you need to poll multiple pins simultaneously, use
    /// [`Gpio::poll_interrupts`] to block while waiting for any of the interrupts to trigger, or switch to
    /// using asynchronous interrupts with [`set_async_interrupt`].
    ///
    /// If `reset` is set to `false`, returns immediately if an interrupt trigger event was cached in a
    /// previous call to `poll_interrupt`.
    /// If `reset` is set to `true`, clears any cached interrupt trigger events before polling.
    ///
    /// The `timeout` duration indicates how long the call will block while waiting
    /// for interrupt trigger events, after which an `Ok(None))` is returned.
    /// `timeout` can be set to `None` to wait indefinitely.
    ///
    /// [`set_interrupt`]: #method.set_interrupt
    /// [`Gpio::poll_interrupts`]: struct.Gpio#method.poll_interrupts
    /// [`set_async_interrupt`]: #method.set_async_interrupt
    pub fn poll_interrupt(
        &mut self,
        reset: bool,
        timeout: Option<Duration>,
    ) -> Result<Option<Level>> {
        let opt =
            (*self.pin.gpio_state.sync_interrupts.lock().unwrap()).poll(&[self], reset, timeout)?;

        if let Some(trigger) = opt {
            Ok(Some(trigger.1))
        } else {
            Ok(None)
        }
    }

    /// Configures an asynchronous interrupt trigger, which will execute the callback on a
    /// separate thread when the interrupt is triggered.
    ///
    /// The callback closure or function pointer is called with a single [`Level`] argument.
    ///
    /// Any previously configured (a)synchronous interrupt triggers will be cleared.
    ///
    /// [`Level`]: enum.Level.html
    pub fn set_async_interrupt<C>(&mut self, trigger: Trigger, callback: C) -> Result<()>
    where
        C: FnMut(Level) + Send + 'static,
    {
        self.clear_interrupt()?;
        self.clear_async_interrupt()?;

        self.async_interrupt = Some(AsyncInterrupt::new(
            self.pin.gpio_state.cdev.as_raw_fd(),
            self.pin.pin,
            trigger,
            callback,
        )?);

        Ok(())
    }

    /// Removes a previously configured asynchronous interrupt trigger.
    pub fn clear_async_interrupt(&mut self) -> Result<()> {
        if let Some(mut interrupt) = self.async_interrupt.take() {
            interrupt.stop()?;
        }

        Ok(())
    }

    impl_clear_on_drop!();
}

impl_drop!(InputPin);

/// GPIO pin configured as output.
#[derive(Debug)]
pub struct OutputPin {
    pin: Pin,
    prev_mode: Option<Mode>,
    clear_on_drop: bool,
}

impl OutputPin {
    pub(crate) fn new(mut pin: Pin) -> OutputPin {
        let prev_mode = pin.mode();

        let prev_mode = if prev_mode == Mode::Output {
            None
        } else {
            pin.set_mode(Mode::Output);
            Some(prev_mode)
        };

        OutputPin {
            pin,
            prev_mode,
            clear_on_drop: true,
        }
    }

    impl_input!();
    impl_output!();
    impl_clear_on_drop!();
}

impl_drop!(OutputPin);

/// GPIO pin configured with an alternate function.
#[derive(Debug)]
pub struct AltPin {
    pin: Pin,
    mode: Mode,
    prev_mode: Option<Mode>,
    clear_on_drop: bool,
}

impl AltPin {
    pub(crate) fn new(mut pin: Pin, mode: Mode) -> AltPin {
        let prev_mode = pin.mode();

        let prev_mode = if prev_mode == mode {
            None
        } else {
            pin.set_mode(mode);
            Some(prev_mode)
        };

        AltPin {
            pin,
            mode,
            prev_mode,
            clear_on_drop: true,
        }
    }

    impl_input!();
    impl_output!();
    impl_clear_on_drop!();
}

impl_drop!(AltPin);

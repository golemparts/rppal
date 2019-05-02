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

use std::os::unix::io::AsRawFd;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use super::soft_pwm::SoftPwm;
use crate::gpio::{interrupt::AsyncInterrupt, GpioState, Level, Mode, PullUpDown, Result, Trigger};

// Maximum GPIO pins on the BCM2835. The actual number of pins
// exposed through the Pi's GPIO header depends on the model.
pub const MAX: usize = 54;

macro_rules! impl_pin {
    () => {
        /// Returns the GPIO pin number.
        ///
        /// Pins are addressed by their BCM numbers, rather than their physical location.
        #[inline]
        pub fn pin(&self) -> u8 {
            self.pin.pin
        }
    }
}

macro_rules! impl_input {
    () => {
        /// Reads the pin's logic level.
        #[inline]
        pub fn read(&self) -> Level {
            self.pin.read()
        }

        /// Reads the pin's logic level, and returns `true` if it's set to [`Low`].
        ///
        /// [`Low`]: enum.Level.html#variant.Low
        #[inline]
        pub fn is_low(&self) -> bool {
            self.pin.read() == Level::Low
        }

        /// Reads the pin's logic level, and returns `true` if it's set to [`High`].
        ///
        /// [`High`]: enum.Level.html#variant.High
        #[inline]
        pub fn is_high(&self) -> bool {
            self.pin.read() == Level::High
        }
    }
}

macro_rules! impl_output {
    () => {
        /// Sets the pin's output state.
        #[inline]
        pub fn write(&mut self, level: Level) {
            self.pin.write(level)
        }

        /// Sets the pin's output state to [`Low`].
        ///
        /// [`Low`]: enum.Level.html#variant.Low
        #[inline]
        pub fn set_low(&mut self) {
            self.pin.set_low()
        }

        /// Sets the pin's output state to [`High`].
        ///
        /// [`High`]: enum.Level.html#variant.High
        #[inline]
        pub fn set_high(&mut self) {
            self.pin.set_high()
        }

        /// Toggles the pin's output state between [`Low`] and [`High`].
        ///
        /// [`Low`]: enum.Level.html#variant.Low
        /// [`High`]: enum.Level.html#variant.High
        #[inline]
        pub fn toggle(&mut self) {
            if self.pin.read() == Level::Low {
                self.set_high();
            } else {
                self.set_low();
            }
        }

        /// Configures a software-based PWM signal.
        ///
        /// `period` indicates the time it takes to complete one cycle.
        ///
        /// `pulse_width` indicates the amount of time the PWM signal is active during a
        /// single period.
        ///
        /// Software-based PWM is inherently inaccurate on a multi-threaded OS due to
        /// scheduling/preemption. If an accurate or faster PWM signal is required, use the
        /// hardware [`Pwm`] peripheral instead. More information can be found [here].
        ///
        /// If `set_pwm` is called when a PWM thread is already active, the existing thread
        /// will be reconfigured at the end of the current cycle.
        ///
        /// [`Pwm`]: ../pwm/struct.Pwm.html
        /// [here]: index.html#software-based-pwm
        pub fn set_pwm(&mut self, period: Duration, pulse_width: Duration) -> Result<()> {
            if let Some(ref mut soft_pwm) = self.soft_pwm {
                soft_pwm.reconfigure(period, pulse_width);
            } else {
                self.soft_pwm = Some(SoftPwm::new(
                    self.pin.pin,
                    self.pin.gpio_state.clone(),
                    period,
                    pulse_width,
                ));
            }

            // Store frequency/duty cycle for the embedded-hal PwmPin implementation.
            #[cfg(feature = "hal")]
            {
                let period_s =
                    period.as_secs() as f64 + (f64::from(period.subsec_nanos()) / 1_000_000_000.0);
                let pulse_width_s = pulse_width.as_secs() as f64
                    + (f64::from(pulse_width.subsec_nanos()) / 1_000_000_000.0);

                if period_s > 0.0 {
                    self.frequency = 1.0 / period_s;
                    self.duty_cycle = (pulse_width_s / period_s).min(1.0);
                } else {
                    self.frequency = 0.0;
                    self.duty_cycle = 0.0;
                }
            }

            Ok(())
        }

        /// Configures a software-based PWM signal.
        ///
        /// `set_pwm_frequency` is a convenience method that converts `frequency` to a period and
        /// `duty_cycle` to a pulse width, and then calls [`set_pwm`].
        ///
        /// `frequency` is specified in hertz (Hz).
        ///
        /// `duty_cycle` is specified as a floating point value between `0.0` (0%) and `1.0` (100%).
        ///
        /// [`set_pwm`]: #method.set_pwm
        pub fn set_pwm_frequency(&mut self, frequency: f64, duty_cycle: f64) -> Result<()> {
            let period = if frequency <= 0.0 {
                0.0
            } else {
                (1.0 / frequency) * 1_000_000_000.0
            };
            let pulse_width = period * duty_cycle.max(0.0).min(1.0);

            self.set_pwm(
                Duration::from_nanos(period as u64),
                Duration::from_nanos(pulse_width as u64),
            )
        }

        /// Stops a previously configured software-based PWM signal.
        ///
        /// The thread responsible for emulating the PWM signal is stopped at the end
        /// of the current cycle.
        pub fn clear_pwm(&mut self) -> Result<()> {
            if let Some(mut soft_pwm) = self.soft_pwm.take() {
                soft_pwm.stop()?;
            }

            Ok(())
        }
    }
}

macro_rules! impl_reset_on_drop {
    () => {
        /// Returns the value of `reset_on_drop`.
        pub fn reset_on_drop(&self) -> bool {
            self.reset_on_drop
        }

        /// When enabled, resets the pin's mode to its original state and disables the
        /// built-in pull-up/pull-down resistors when the pin goes out of scope.
        /// By default, this is set to `true`.
        ///
        /// ## Note
        ///
        /// Drop methods aren't called when a process is abnormally terminated, for
        /// instance when a user presses <kbd>Ctrl</kbd> + <kbd>C</kbd>, and the `SIGINT` signal
        /// isn't caught. You can catch those using crates such as [`simple_signal`].
        ///
        /// [`simple_signal`]: https://crates.io/crates/simple-signal
        pub fn set_reset_on_drop(&mut self, reset_on_drop: bool) {
            self.reset_on_drop = reset_on_drop;
        }
    };
}

macro_rules! impl_drop {
    ($struct:ident) => {
        impl Drop for $struct {
            /// Resets the pin's mode and disables the built-in pull-up/pull-down
            /// resistors if `reset_on_drop` is set to `true` (default).
            fn drop(&mut self) {
                if !self.reset_on_drop {
                    return;
                }

                if let Some(prev_mode) = self.prev_mode {
                    self.pin.set_mode(prev_mode);
                }

                if self.pud_mode != PullUpDown::Off {
                    self.pin.set_pullupdown(PullUpDown::Off);
                }
            }
        }
    };
}

macro_rules! impl_eq {
    ($struct:ident) => {
        impl PartialEq for $struct {
            fn eq(&self, other: &$struct) -> bool {
                self.pin == other.pin
            }
        }

        impl<'a> PartialEq<&'a $struct> for $struct {
            fn eq(&self, other: &&'a $struct) -> bool {
                self.pin == other.pin
            }
        }

        impl<'a> PartialEq<$struct> for &'a $struct {
            fn eq(&self, other: &$struct) -> bool {
                self.pin == other.pin
            }
        }

        impl Eq for $struct {}
    };
}

/// Unconfigured GPIO pin.
///
/// `Pin`s are constructed by retrieving them using [`Gpio::get`].
///
/// An unconfigured `Pin` can be used to read the pin's mode and logic level.
/// Converting the `Pin` to an [`InputPin`], [`OutputPin`] or [`IoPin`] through the
/// various `into_` methods available on `Pin` configures the appropriate mode, and
/// provides access to additional methods relevant to the selected pin mode.
///
/// The `unproven` `embedded-hal` [`digital::InputPin`] trait implementation for `Pin` can be enabled
/// by specifying the optional `hal-unproven` feature in the dependency declaration for
/// the `rppal` crate.
///
/// [`digital::InputPin`]: ../../embedded_hal/digital/trait.InputPin.html
/// [`Gpio::get`]: struct.Gpio.html#method.get
/// [`InputPin`]: struct.InputPin.html
/// [`OutputPin`]: struct.OutputPin.html
/// [`IoPin`]: struct.IoPin.html
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

    /// Returns the GPIO pin number.
    ///
    /// Pins are addressed by their BCM numbers, rather than their physical location.
    #[inline]
    pub fn pin(&self) -> u8 {
        self.pin
    }

    /// Returns the pin's mode.
    #[inline]
    pub fn mode(&self) -> Mode {
        self.gpio_state.gpio_mem.mode(self.pin)
    }

    /// Reads the pin's logic level.
    #[inline]
    pub fn read(&self) -> Level {
        self.gpio_state.gpio_mem.level(self.pin)
    }

    /// Consumes the `Pin`, returns an [`InputPin`], sets its mode to [`Input`],
    /// and disables the pin's built-in pull-up/pull-down resistors.
    ///
    /// [`InputPin`]: struct.InputPin.html
    /// [`Input`]: enum.Mode.html#variant.Input
    #[inline]
    pub fn into_input(self) -> InputPin {
        InputPin::new(self, PullUpDown::Off)
    }

    /// Consumes the `Pin`, returns an [`InputPin`], sets its mode to [`Input`],
    /// and enables the pin's built-in pull-down resistor.
    ///
    /// The pull-down resistor is disabled when `InputPin` goes out of scope if [`reset_on_drop`]
    /// is set to `true` (default).
    ///
    /// [`InputPin`]: struct.InputPin.html
    /// [`Input`]: enum.Mode.html#variant.Input
    /// [`reset_on_drop`]: struct.InputPin.html#method.set_reset_on_drop
    #[inline]
    pub fn into_input_pulldown(self) -> InputPin {
        InputPin::new(self, PullUpDown::PullDown)
    }

    /// Consumes the `Pin`, returns an [`InputPin`], sets its mode to [`Input`],
    /// and enables the pin's built-in pull-up resistor.
    ///
    /// The pull-up resistor is disabled when `InputPin` goes out of scope if [`reset_on_drop`]
    /// is set to `true` (default).
    ///
    /// [`InputPin`]: struct.InputPin.html
    /// [`Input`]: enum.Mode.html#variant.Input
    /// [`reset_on_drop`]: struct.InputPin.html#method.set_reset_on_drop
    #[inline]
    pub fn into_input_pullup(self) -> InputPin {
        InputPin::new(self, PullUpDown::PullUp)
    }

    /// Consumes the `Pin`, returns an [`OutputPin`] and sets its mode to [`Output`].
    ///
    /// [`OutputPin`]: struct.OutputPin.html
    /// [`Output`]: enum.Mode.html#variant.Output
    #[inline]
    pub fn into_output(self) -> OutputPin {
        OutputPin::new(self)
    }

    /// Consumes the `Pin`, returns an [`IoPin`] and sets its mode to the specified mode.
    ///
    /// [`IoPin`]: struct.IoPin.html
    /// [`Mode`]: enum.Mode.html
    #[inline]
    pub fn into_io(self, mode: Mode) -> IoPin {
        IoPin::new(self, mode)
    }

    #[inline]
    pub(crate) fn set_mode(&mut self, mode: Mode) {
        self.gpio_state.gpio_mem.set_mode(self.pin, mode);
    }

    #[inline]
    pub(crate) fn set_pullupdown(&mut self, pud: PullUpDown) {
        self.gpio_state.gpio_mem.set_pullupdown(self.pin, pud);
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

impl_eq!(Pin);

/// GPIO pin configured as input.
///
/// `InputPin`s are constructed by converting a [`Pin`] using [`Pin::into_input`],
/// [`Pin::into_input_pullup`] or [`Pin::into_input_pulldown`]. The pin's mode is
/// automatically set to [`Input`].
///
/// An `InputPin` can be used to read a pin's logic level, or (a)synchronously poll for
/// interrupt trigger events.
///
/// The `unproven` `embedded-hal` [`digital::InputPin`] trait implementation for `InputPin` can be enabled
/// by specifying the optional `hal-unproven` feature in the dependency declaration for
/// the `rppal` crate.
///
/// [`digital::InputPin`]: ../../embedded_hal/digital/trait.InputPin.html
/// [`Pin`]: struct.Pin.html
/// [`Input`]: enum.Mode.html#variant.Input
/// [`Pin::into_input`]: struct.Pin.html#method.into_input
/// [`Pin::into_input_pullup`]: struct.Pin.html#method.into_input_pullup
/// [`Pin::into_input_pulldown`]: struct.Pin.html#method.into_input_pulldown
#[derive(Debug)]
pub struct InputPin {
    pub(crate) pin: Pin,
    prev_mode: Option<Mode>,
    async_interrupt: Option<AsyncInterrupt>,
    reset_on_drop: bool,
    pud_mode: PullUpDown,
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
            reset_on_drop: true,
            pud_mode,
        }
    }

    impl_pin!();
    impl_input!();

    /// Configures a synchronous interrupt trigger.
    ///
    /// After configuring a synchronous interrupt trigger, call [`poll_interrupt`] or
    /// [`Gpio::poll_interrupts`] to block while waiting for a trigger event.
    ///
    /// Any previously configured (a)synchronous interrupt triggers will be cleared.
    ///
    /// [`poll_interrupt`]: #method.poll_interrupt
    /// [`Gpio::poll_interrupts`]: struct.Gpio.html#method.poll_interrupts
    pub fn set_interrupt(&mut self, trigger: Trigger) -> Result<()> {
        self.clear_async_interrupt()?;

        // Each pin can only be configured for a single trigger type
        (*self.pin.gpio_state.sync_interrupts.lock().unwrap()).set_interrupt(self.pin(), trigger)
    }

    /// Removes a previously configured synchronous interrupt trigger.
    pub fn clear_interrupt(&mut self) -> Result<()> {
        (*self.pin.gpio_state.sync_interrupts.lock().unwrap()).clear_interrupt(self.pin())
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
    /// Setting `reset` to `false` returns any cached interrupt trigger events if available. Setting `reset` to `true`
    /// clears all cached events before polling for new events.
    ///
    /// The `timeout` duration indicates how long the call will block while waiting
    /// for interrupt trigger events, after which an `Ok(None))` is returned.
    /// `timeout` can be set to `None` to wait indefinitely.
    ///
    /// [`set_interrupt`]: #method.set_interrupt
    /// [`Gpio::poll_interrupts`]: struct.Gpio.html#method.poll_interrupts
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

    /// Configures an asynchronous interrupt trigger, which executes the callback on a
    /// separate thread when the interrupt is triggered.
    ///
    /// The callback closure or function pointer is called with a single [`Level`] argument.
    ///
    /// Any previously configured (a)synchronous interrupt triggers for this pin are cleared
    /// when `set_async_interrupt` is called, or when `InputPin` goes out of scope.
    ///
    /// [`clear_async_interrupt`]: #method.clear_async_interrupt
    /// [`Level`]: enum.Level.html
    pub fn set_async_interrupt<C>(&mut self, trigger: Trigger, callback: C) -> Result<()>
    where
        C: FnMut(Level) + Send + 'static,
    {
        self.clear_interrupt()?;
        self.clear_async_interrupt()?;

        self.async_interrupt = Some(AsyncInterrupt::new(
            self.pin.gpio_state.cdev.as_raw_fd(),
            self.pin(),
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

    impl_reset_on_drop!();
}

impl_drop!(InputPin);
impl_eq!(InputPin);

/// GPIO pin configured as output.
///
/// `OutputPin`s are constructed by converting a [`Pin`] using [`Pin::into_output`].
/// The pin's mode is automatically set to [`Output`].
///
/// An `OutputPin` can be used to change a pin's output state.
///
/// The `embedded-hal` [`digital::OutputPin`] and [`PwmPin`] trait implementations for `OutputPin`
/// can be enabled by specifying the optional `hal` feature in the dependency
/// declaration for the `rppal` crate.
///
/// The `unproven` `embedded-hal` [`digital::InputPin`], [`digital::StatefulOutputPin`],
/// [`digital::ToggleableOutputPin`] and [`Pwm`] trait implementations for `OutputPin` can be enabled
/// by specifying the optional `hal-unproven` feature in the dependency declaration for
/// the `rppal` crate.
///
/// [`digital::InputPin`]: ../../embedded_hal/digital/trait.InputPin.html
/// [`digital::StatefulOutputPin`]: ../../embedded_hal/digital/trait.StatefulOutputPin.html
/// [`digital::ToggleableOutputPin`]: ../../embedded_hal/digital/trait.ToggleableOutputPin.html
/// [`Pwm`]: ../../embedded_hal/trait.Pwm.html
/// [`Pin`]: struct.Pin.html
/// [`Output`]: enum.Mode.html#variant.Output
/// [`Pin::into_output`]: struct.Pin.html#method.into_output
/// [`digital::OutputPin`]: ../../embedded_hal/digital/trait.OutputPin.html
/// [`PwmPin`]: ../../embedded_hal/trait.PwmPin.html
#[derive(Debug)]
pub struct OutputPin {
    pin: Pin,
    prev_mode: Option<Mode>,
    reset_on_drop: bool,
    pud_mode: PullUpDown,
    pub(crate) soft_pwm: Option<SoftPwm>,
    // Stores the softpwm frequency. Used for embedded_hal::PwmPin.
    #[cfg(feature = "hal")]
    pub(crate) frequency: f64,
    // Stores the softpwm duty cycle. Used for embedded_hal::PwmPin.
    #[cfg(feature = "hal")]
    pub(crate) duty_cycle: f64,
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
            reset_on_drop: true,
            pud_mode: PullUpDown::Off,
            soft_pwm: None,
            #[cfg(feature = "hal")]
            frequency: 0.0,
            #[cfg(feature = "hal")]
            duty_cycle: 0.0,
        }
    }

    impl_pin!();

    /// Returns `true` if the pin's output state is set to [`Low`].
    ///
    /// [`Low`]: enum.Level.html#variant.Low
    #[inline]
    pub fn is_set_low(&self) -> bool {
        self.pin.read() == Level::Low
    }

    /// Returns `true` if the pin's output state is set to [`High`].
    ///
    /// [`High`]: enum.Level.html#variant.High
    #[inline]
    pub fn is_set_high(&self) -> bool {
        self.pin.read() == Level::High
    }

    impl_output!();
    impl_reset_on_drop!();
}

impl_drop!(OutputPin);
impl_eq!(OutputPin);

/// GPIO pin that can be (re)configured for any mode or alternate function.
///
/// `IoPin`s are constructed by converting a [`Pin`] using [`Pin::into_io`].
/// The pin's mode is automatically set to the specified mode.
///
/// An `IoPin` can be reconfigured for any available mode. Depending on the
/// mode, some methods may not have any effect. For instance, calling a method that
/// alters the pin's output state won't cause any changes when the pin's mode is set
/// to [`Input`].
///
/// The `embedded-hal` [`digital::OutputPin`] and [`PwmPin`] trait implementations for `IoPin`
/// can be enabled by specifying the optional `hal` feature in the dependency
/// declaration for the `rppal` crate.
///
/// The `unproven` `embedded-hal` [`digital::InputPin`], [`digital::StatefulOutputPin`],
/// [`digital::ToggleableOutputPin`] and [`Pwm`] trait implementations for `IoPin` can be enabled
/// by specifying the optional `hal-unproven` feature in the dependency declaration for
/// the `rppal` crate.
///
/// [`digital::InputPin`]: ../../embedded_hal/digital/trait.InputPin.html
/// [`digital::StatefulOutputPin`]: ../../embedded_hal/digital/trait.StatefulOutputPin.html
/// [`digital::ToggleableOutputPin`]: ../../embedded_hal/digital/trait.ToggleableOutputPin.html
/// [`Pwm`]: ../../embedded_hal/trait.Pwm.html
/// [`Pin`]: struct.Pin.html
/// [`Input`]: enum.Mode.html#variant.Input
/// [`Pin::into_io`]: struct.Pin.html#method.into_io
/// [`digital::OutputPin`]: ../../embedded_hal/digital/trait.OutputPin.html
/// [`PwmPin`]: ../../embedded_hal/trait.PwmPin.html
#[derive(Debug)]
pub struct IoPin {
    pin: Pin,
    mode: Mode,
    prev_mode: Option<Mode>,
    reset_on_drop: bool,
    pud_mode: PullUpDown,
    pub(crate) soft_pwm: Option<SoftPwm>,
    // Stores the softpwm frequency. Used for embedded_hal::PwmPin.
    #[cfg(feature = "hal")]
    pub(crate) frequency: f64,
    // Stores the softpwm duty cycle. Used for embedded_hal::PwmPin.
    #[cfg(feature = "hal")]
    pub(crate) duty_cycle: f64,
}

impl IoPin {
    pub(crate) fn new(mut pin: Pin, mode: Mode) -> IoPin {
        let prev_mode = pin.mode();

        let prev_mode = if prev_mode == mode {
            None
        } else {
            pin.set_mode(mode);
            Some(prev_mode)
        };

        IoPin {
            pin,
            mode,
            prev_mode,
            reset_on_drop: true,
            pud_mode: PullUpDown::Off,
            soft_pwm: None,
            #[cfg(feature = "hal")]
            frequency: 0.0,
            #[cfg(feature = "hal")]
            duty_cycle: 0.0,
        }
    }

    impl_pin!();

    /// Returns the pin's mode.
    #[inline]
    pub fn mode(&self) -> Mode {
        self.pin.mode()
    }

    /// Sets the pin's mode.
    #[inline]
    pub fn set_mode(&mut self, mode: Mode) {
        // If self.prev_mode is set to None, that means the
        // requested mode during construction was the same as
        // the current mode. Save that mode if we're changing
        // it to something else now, so we can reset it on drop.
        if self.prev_mode.is_none() && mode != self.mode {
            self.prev_mode = Some(self.mode);
        }

        self.pin.set_mode(mode);
    }

    /// Configures the built-in pull-up/pull-down resistors.
    #[inline]
    pub fn set_pullupdown(&mut self, pud: PullUpDown) {
        self.pin.set_pullupdown(pud);
        self.pud_mode = pud;
    }

    impl_input!();
    impl_output!();
    impl_reset_on_drop!();
}

impl_drop!(IoPin);
impl_eq!(IoPin);

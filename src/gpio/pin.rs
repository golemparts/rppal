use std::fs::File;
use std::os::unix::io::AsRawFd;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::gpio::{Result, Mode, Level, Trigger, PullUpDown, mem::GpioMem, interrupt::{AsyncInterrupt, EventLoop}};

// Maximum GPIO pins on the BCM2835. The actual number of pins
// exposed through the Pi's GPIO header depends on the model.
pub const MAX: usize = 54;

#[derive(Debug)]
pub struct Pin {
    pub(crate) pin: u8,
    event_loop: Arc<Mutex<EventLoop>>,
    gpio_mem: Arc<GpioMem>,
    gpio_cdev: Arc<File>,
}

impl Pin {
    pub(crate) fn new(pin: u8, event_loop: Arc<Mutex<EventLoop>>, gpio_mem: Arc<GpioMem>, gpio_cdev: Arc<File>) -> Pin {
        Pin { pin, event_loop, gpio_mem, gpio_cdev }
    }

    pub fn as_input(&mut self) -> InputPin {
        InputPin::new(self)
    }

    pub fn as_output(&mut self) -> OutputPin {
        OutputPin::new(self, Mode::Output)
    }

    pub fn as_output_with_mode(&mut self, mode: Mode) -> OutputPin {
        OutputPin::new(self, mode)
    }

    pub(crate) fn set_mode(&mut self, mode: Mode) {
        (*self.gpio_mem).set_mode(self.pin, mode);
    }

    /// Returns the current GPIO pin mode.
    pub fn mode(&self) -> Mode {
        (*self.gpio_mem).mode(self.pin)
    }

    /// Configures the built-in GPIO pull-up/pull-down resistors.
    pub fn set_pullupdown(&self, pud: PullUpDown) {
        (*self.gpio_mem).set_pullupdown(self.pin, pud);
    }
}

#[derive(Debug)]
pub struct InputPin<'a> {
    pub(crate) pin: &'a mut Pin,
    prev_mode: Option<Mode>,
    async_interrupt: Option<AsyncInterrupt>,
    clear_on_drop: bool,
}

impl<'a> InputPin<'a> {
    pub(crate) fn new(pin: &'a mut Pin) -> InputPin<'a> {
        let prev_mode = pin.mode();

        let prev_mode = if prev_mode == Mode::Input {
            None
        } else {
            pin.set_mode(Mode::Input);
            Some(prev_mode)
        };

        InputPin { pin, prev_mode, async_interrupt: None, clear_on_drop: true }
    }

    /// Returns the value of `clear_on_drop`.
    pub fn clear_on_drop(&self) -> bool {
        self.clear_on_drop
    }

    /// When enabled, resets all pins to their original state when `Gpio` goes out of scope.
    ///
    /// Drop methods aren't called when a program is abnormally terminated,
    /// for instance when a user presses Ctrl-C, and the SIGINT signal isn't
    /// caught. You'll either have to catch those using crates such as
    /// [`simple_signal`], or manually call [`cleanup`].
    ///
    /// By default, `clear_on_drop` is set to `true`.
    ///
    /// [`simple_signal`]: https://crates.io/crates/simple-signal
    /// [`cleanup`]: #method.cleanup
    pub fn set_clear_on_drop(&mut self, clear_on_drop: bool) {
        self.clear_on_drop = clear_on_drop;
    }

    pub fn read(&self) -> Level {
        (*self.pin.gpio_mem).level(self.pin.pin)
    }

    /// Configures a synchronous interrupt trigger.
    ///
    /// After configuring a synchronous interrupt trigger, you can use
    /// [`poll_interrupt`] to wait for a trigger event.
    ///
    /// `set_interrupt` will remove any previously configured
    /// (a)synchronous interrupt triggers for the same pin.
    ///
    /// [`poll_interrupt`]: #method.poll_interrupt
    pub fn set_interrupt(&mut self, trigger: Trigger) -> Result<()> {
        self.clear_async_interrupt()?;

        // Each pin can only be configured for a single trigger type
        (*self.pin.event_loop.lock().unwrap()).set_interrupt(self.pin.pin, trigger)
    }

    /// Removes a previously configured synchronous interrupt trigger.
    pub fn clear_interrupt(&mut self) -> Result<()> {
        (*self.pin.event_loop.lock().unwrap()).clear_interrupt(self.pin.pin)
    }

    /// Blocks until an interrupt is triggered on the specified pin, or a timeout occurs.
    ///
    /// `poll_interrupt` only works for pins that have been configured for synchronous interrupts using
    /// [`set_interrupt`]. Asynchronous interrupt triggers are automatically polled on a separate thread.
    ///
    /// Setting `reset` to `false` causes `poll_interrupt` to return immediately if the interrupt
    /// has been triggered since the previous call to [`set_interrupt`] or `poll_interrupt`.
    /// Setting `reset` to `true` clears any cached trigger events for the pin.
    ///
    /// The `timeout` duration indicates how long the call to `poll_interrupt` will block while waiting
    /// for interrupt trigger events, after which an `Ok(None))` is returned.
    /// `timeout` can be set to `None` to wait indefinitely.
    ///
    /// [`set_interrupt`]: #method.set_interrupt
    pub fn poll_interrupt(&mut self, reset: bool, timeout: Option<Duration>) -> Result<Option<Level>> {
        let opt = (*self.pin.event_loop.lock().unwrap()).poll(&[self.pin.pin], reset, timeout)?;

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
    /// `set_async_interrupt` will remove any previously configured
    /// (a)synchronous interrupt triggers for the same pin.
    ///
    /// [`Level`]: enum.Level.html
    pub fn set_async_interrupt<C>(&mut self, trigger: Trigger, callback: C) -> Result<()>
    where
        C: FnMut(Level) + Send + 'static,
    {
        self.clear_interrupt()?;
        self.clear_async_interrupt()?;

        self.async_interrupt = Some(AsyncInterrupt::new(
            (*self.pin.gpio_cdev).as_raw_fd(),
            self.pin.pin,
            trigger,
            callback,
        )?);

        Ok(())
    }

    pub(crate) fn clear_async_interrupt(&mut self) -> Result<()> {
        if let Some(mut interrupt) = self.async_interrupt.take() {
            interrupt.stop()?;
        }

        Ok(())
    }
}

impl<'a> Drop for InputPin<'a> {
    fn drop(&mut self) {
        let _ = self.clear_async_interrupt();

        if self.clear_on_drop == false {
          return
        }

        if let Some(prev_mode) = self.prev_mode {
            self.pin.set_mode(prev_mode)
        }
    }
}

#[derive(Debug)]
pub struct OutputPin<'a> {
    pin: &'a mut Pin,
    mode: Mode,
    prev_mode: Option<Mode>,
    clear_on_drop: bool,
}

impl<'a> OutputPin<'a> {
    pub(crate) fn new(pin: &'a mut Pin, mode: Mode) -> OutputPin<'a> {
        let prev_mode = pin.mode();

        let prev_mode = if prev_mode == mode {
            None
        } else {
            pin.set_mode(mode);
            Some(prev_mode)
        };

        OutputPin { pin, mode, prev_mode, clear_on_drop: true }
    }

    /// Returns the value of `clear_on_drop`.
    pub fn clear_on_drop(&self) -> bool {
        self.clear_on_drop
    }

    /// When enabled, resets all pins to their original state when `Gpio` goes out of scope.
    ///
    /// Drop methods aren't called when a program is abnormally terminated,
    /// for instance when a user presses Ctrl-C, and the SIGINT signal isn't
    /// caught. You'll either have to catch those using crates such as
    /// [`simple_signal`], or manually call [`cleanup`].
    ///
    /// By default, `clear_on_drop` is set to `true`.
    ///
    /// [`simple_signal`]: https://crates.io/crates/simple-signal
    /// [`cleanup`]: #method.cleanup
    pub fn set_clear_on_drop(&mut self, clear_on_drop: bool) {
        self.clear_on_drop = clear_on_drop;
    }

    pub fn set_low(&mut self) {
        (*self.pin.gpio_mem).set_low(self.pin.pin);
    }

    pub fn set_high(&mut self) {
        (*self.pin.gpio_mem).set_high(self.pin.pin);
    }

    pub fn write(&mut self, level: Level) {
        match level {
            Level::Low => self.set_low(),
            Level::High => self.set_high(),
        };
    }
}

impl<'a> Drop for OutputPin<'a> {
  fn drop(&mut self) {
    if self.clear_on_drop == false {
      return
    }

    if let Some(prev_mode) = self.prev_mode {
      self.pin.set_mode(prev_mode)
    }
  }
}

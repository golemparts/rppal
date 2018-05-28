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

use std::fmt;
use std::fs::File;
use std::io;
use std::io::{Read, Seek, SeekFrom};
use std::os::unix::io::AsRawFd;
use std::result;
use std::sync::mpsc::TryRecvError;
use std::thread;
use std::time::{Duration, Instant};

use mio::event::Evented;
use mio::unix::{EventedFd, UnixReady};
use mio::{Events, Poll, PollOpt, Ready, Token};
use mio_extras::channel;

use gpio::sysfs;

pub use gpio::sysfs::Direction;
pub use gpio::{Level, Trigger};

quick_error! {
/// Errors that can occur while working with interrupts.
    #[derive(Debug)]
    pub enum Error {
/// Synchronous interrupt isn't initialized.
        NotInitialized { description("not initialized") }
/// Time out.
        TimeOut { description("interrupt polling timed out while waiting for a trigger") }
/// IO error.
        Io(err: io::Error) { description(err.description()) from() }
/// Disconnected while sending a control message to the interrupt polling thread.
        SendDisconnected { description("receiving half of the channel has disconnected") }
/// Interrupt polling thread panicked.
        ThreadPanic { description("interrupt polling thread panicked") }
    }
}

impl<T> From<channel::SendError<T>> for Error {
    fn from(err: channel::SendError<T>) -> Error {
        match err {
            channel::SendError::Io(e) => Error::Io(e),
            channel::SendError::Disconnected(_) => Error::SendDisconnected,
        }
    }
}

/// Result type returned from methods that can have `rppal::gpio::interrupt::Error`s.
pub type Result<T> = result::Result<T, Error>;

const TOKEN_RX: usize = 0;
const TOKEN_PIN: usize = 1;

#[derive(Debug)]
struct Interrupt {
    pin: u8,
    trigger: Trigger,
    sysfs_value: File,
}

impl Interrupt {
    fn new(pin: u8, trigger: Trigger) -> Result<Interrupt> {
        // Export the GPIO pin so we can configure it through sysfs, set its mode to
        // input, and set the trigger type.
        sysfs::export(pin)?;
        sysfs::set_direction(pin, Direction::In)?;
        sysfs::set_edge(pin, trigger)?;

        Ok(Interrupt {
            pin,
            trigger,
            sysfs_value: sysfs::open_value(pin)?,
        })
    }

    fn trigger(&self) -> Trigger {
        self.trigger
    }

    fn set_trigger(&mut self, trigger: Trigger) -> Result<()> {
        self.trigger = trigger;
        sysfs::set_edge(self.pin, trigger)?;

        Ok(())
    }

    fn level(&mut self) -> Result<Level> {
        let mut buffer = [0; 1];
        self.sysfs_value.read_exact(&mut buffer)?;
        self.sysfs_value.seek(SeekFrom::Start(0))?;

        match &buffer {
            b"0" => Ok(Level::Low),
            _ => Ok(Level::High),
        }
    }
}

impl Drop for Interrupt {
    fn drop(&mut self) {
        sysfs::unexport(self.pin).ok();
    }
}

impl Evented for Interrupt {
    fn register(
        &self,
        poll: &Poll,
        token: Token,
        interest: Ready,
        opts: PollOpt,
    ) -> io::Result<()> {
        EventedFd(&self.sysfs_value.as_raw_fd()).register(poll, token, interest, opts)
    }

    fn reregister(
        &self,
        poll: &Poll,
        token: Token,
        interest: Ready,
        opts: PollOpt,
    ) -> io::Result<()> {
        EventedFd(&self.sysfs_value.as_raw_fd()).reregister(poll, token, interest, opts)
    }

    fn deregister(&self, poll: &Poll) -> io::Result<()> {
        EventedFd(&self.sysfs_value.as_raw_fd()).deregister(poll)
    }
}

#[derive(Debug)]
struct TriggerStatus {
    interrupt: Option<Interrupt>,
    triggered: bool,
    level: Level,
}

#[derive(Debug)]
pub struct EventLoop {
    poll: Poll,
    events: Events,
    trigger_status: Vec<TriggerStatus>,
}

impl EventLoop {
    pub fn new(capacity: usize) -> Result<EventLoop> {
        let mut trigger_status = Vec::with_capacity(capacity);

        // Initialize trigger_status while circumventing the Copy/Clone requirement
        for _ in 0..trigger_status.capacity() {
            trigger_status.push(TriggerStatus {
                interrupt: None,
                triggered: false,
                level: Level::Low,
            });
        }

        Ok(EventLoop {
            poll: Poll::new()?,
            events: Events::with_capacity(capacity),
            trigger_status,
        })
    }

    pub fn poll(
        &mut self,
        pins: &[u8],
        reset: bool,
        timeout: Option<Duration>,
    ) -> Result<(u8, Level)> {
        for pin in pins {
            if *pin as usize >= self.trigger_status.capacity() {
                return Err(Error::NotInitialized);
            }

            // Did we cache any trigger events during the previous poll?
            if self.trigger_status[*pin as usize].triggered {
                self.trigger_status[*pin as usize].triggered = false;

                if !reset {
                    return Ok((*pin, self.trigger_status[*pin as usize].level));
                }
            }

            // Read the logic level to reset any pending trigger events
            if let Some(ref mut interrupt) = self.trigger_status[*pin as usize].interrupt {
                if reset {
                    interrupt.level()?;
                }
            }
        }

        // Loop until we get any of the events we're waiting for, or a timeout occurs
        let now = Instant::now();
        loop {
            self.poll.poll(&mut self.events, timeout)?;

            // No events means a timeout occurred
            if self.events.is_empty() {
                return Err(Error::TimeOut);
            }

            for event in &self.events {
                if event.token().0 < self.trigger_status.capacity()
                    && event.readiness().is_readable()
                    && UnixReady::from(event.readiness()).is_error()
                {
                    self.trigger_status[event.token().0].triggered = true;
                    self.trigger_status[event.token().0].level = if let Some(ref mut interrupt) =
                        self.trigger_status[event.token().0].interrupt
                    {
                        interrupt.level()?
                    } else {
                        Level::Low
                    };
                }
            }

            // Were any interrupts triggered? If so, return one. The rest
            // will be saved for the next poll.
            for pin in pins {
                if self.trigger_status[*pin as usize].triggered {
                    self.trigger_status[*pin as usize].triggered = false;
                    return Ok((*pin, self.trigger_status[*pin as usize].level));
                }
            }

            // It's possible a pin we're not waiting for continuously triggers
            // an interrupt, causing repeated loops with calls to poll() using a
            // reset timeout value. Make sure we haven't been looping longer than
            // the requested timeout.
            if let Some(t) = timeout {
                if now.elapsed() > t {
                    return Err(Error::TimeOut);
                }
            }
        }
    }

    pub fn set_interrupt(&mut self, pin: u8, trigger: Trigger) -> Result<()> {
        self.trigger_status[pin as usize].triggered = false;

        // Interrupt already exists. We just need to change the trigger.
        if let Some(ref mut interrupt) = self.trigger_status[pin as usize].interrupt {
            if interrupt.trigger != trigger {
                interrupt.set_trigger(trigger)?;
            }

            return Ok(());
        }

        // Register a new interrupt
        let mut base = Interrupt::new(pin, trigger)?;

        base.level()?;
        self.poll.register(
            &base,
            Token(pin as usize),
            Ready::readable() | UnixReady::error(),
            PollOpt::edge(),
        )?;

        self.trigger_status[pin as usize].interrupt = Some(base);

        Ok(())
    }

    pub fn clear_interrupt(&mut self, pin: u8) -> Result<()> {
        self.trigger_status[pin as usize].triggered = false;

        if let Some(interrupt) = self.trigger_status[pin as usize].interrupt.take() {
            self.poll.deregister(&interrupt)?;
        }

        Ok(())
    }
}

enum ControlMsg {
    Stop,
}

pub struct AsyncInterrupt {
    pin: u8,
    poll_thread: Option<thread::JoinHandle<Result<()>>>,
    tx: channel::Sender<ControlMsg>,
}

impl AsyncInterrupt {
    pub fn new<C>(pin: u8, trigger: Trigger, mut callback: C) -> Result<AsyncInterrupt>
    where
        C: FnMut(Level) + Send + 'static,
    {
        let (tx, rx) = channel::channel();

        let poll_thread = thread::spawn(move || -> Result<()> {
            let poll = Poll::new()?;
            let mut events = Events::with_capacity(2);

            poll.register(&rx, Token(TOKEN_RX), Ready::readable(), PollOpt::edge())?;

            let mut base = Interrupt::new(pin, trigger)?;
            base.level()?;
            poll.register(
                &base,
                Token(TOKEN_PIN),
                Ready::readable() | UnixReady::error(),
                PollOpt::edge(),
            )?;

            loop {
                poll.poll(&mut events, None)?;

                for event in &events {
                    if event.token() == Token(TOKEN_RX) {
                        match rx.try_recv() {
                            Ok(ControlMsg::Stop) => {
                                return Ok(());
                            }
                            Err(TryRecvError::Disconnected) => {
                                return Ok(());
                            }
                            Err(TryRecvError::Empty) => {
                                break;
                            }
                        }
                    } else if event.token() == Token(TOKEN_PIN)
                        && event.readiness().is_readable()
                        && UnixReady::from(event.readiness()).is_error()
                    {
                        let interrupt_value = base.level()?;

                        callback(interrupt_value);
                    }
                }
            }
        });

        Ok(AsyncInterrupt {
            pin,
            poll_thread: Some(poll_thread),
            tx,
        })
    }

    pub fn stop(&mut self) -> Result<()> {
        self.tx.send(ControlMsg::Stop)?;

        if let Some(poll_thread) = self.poll_thread.take() {
            match poll_thread.join() {
                Ok(r) => return r,
                Err(_) => return Err(Error::ThreadPanic),
            }
        }

        Ok(())
    }
}

impl Drop for AsyncInterrupt {
    fn drop(&mut self) {
        // Unexport the pin here, because we can't rely on the thread
        // living long enough to unexport it.
        sysfs::unexport(self.pin).ok();
    }
}

impl fmt::Debug for AsyncInterrupt {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("AsyncInterrupt")
            .field("pin", &self.pin)
            .field("poll_thread", &self.poll_thread)
            .field("tx", &"")
            .finish()
    }
}

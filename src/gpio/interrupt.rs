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

use std::fs::File;
use std::io;
use std::io::{Read, Seek, SeekFrom};
use std::os::unix::io::AsRawFd;
use std::result;
use std::sync::mpsc::TryRecvError;
use std::thread;
use std::time::Duration;

use mio::event::Evented;
use mio::unix::{EventedFd, UnixReady};
use mio::{Events, Poll, PollOpt, Ready, Token};
use mio_extras::channel;

use gpio::sysfs;

pub use gpio::sysfs::Direction;
pub use gpio::{Level, Trigger};

quick_error! {
    #[derive(Debug)]
/// Errors that can occur while working with interrupts.
    pub enum Error {
/// Synchronous interrupt isn't initialized.
        NotInitialized { description("not initialized") }
/// Time out.
        TimeOut { description("interrupt polling timed out while waiting for a trigger") }
/// IO error.
        Io(err: io::Error) { description(err.description()) from() }
/// Sysfs error.
        Sysfs(err: sysfs::Error) { description(err.description()) from() }
/// IO error while communicating with the interrupt polling thread.
        SendIo(err: io::Error) { description(err.description()) }
/// Disconnected while sending a control message to the interrupt polling thread.
        SendDisconnected { description("receiving half of the channel has disconnected") }
/// Interrupt polling thread panicked.
        ThreadPanic { description("interrupt polling thread panicked") }
    }
}

impl<T> From<channel::SendError<T>> for Error {
    fn from(err: channel::SendError<T>) -> Error {
        match err {
            channel::SendError::Io(e) => Error::SendIo(e),
            channel::SendError::Disconnected(_) => Error::SendDisconnected,
        }
    }
}

/// Result type returned from methods that can have `rppal::gpio::interrupt::Error`s.
pub type Result<T> = result::Result<T, Error>;

const TOKEN_RX: usize = 0;
const TOKEN_PIN: usize = 1;

struct InterruptBase {
    pin: u8,
    trigger: Trigger,
    sysfs_value: File,
}

impl InterruptBase {
    fn new(pin: u8, trigger: Trigger) -> Result<InterruptBase> {
        // Export the GPIO pin so we can configure it through sysfs, set its mode to
        // input, and set the trigger type.
        sysfs::export(pin)?;
        sysfs::set_direction(pin, Direction::In)?;
        sysfs::set_edge(pin, trigger)?;

        Ok(InterruptBase {
            pin,
            trigger,
            sysfs_value: sysfs::open_value(pin)?,
        })
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

impl Drop for InterruptBase {
    fn drop(&mut self) {
        sysfs::unexport(self.pin).ok();
    }
}

impl Evented for InterruptBase {
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

pub struct Interrupt {
    base: InterruptBase,
    poll: Poll,
    events: Events,
}

impl Interrupt {
    pub fn new(pin: u8, trigger: Trigger) -> Result<Interrupt> {
        let mut base = InterruptBase::new(pin, trigger)?;
        let poll = Poll::new()?;
        let events = Events::with_capacity(1);

        base.level()?;
        poll.register(
            &base,
            Token(TOKEN_PIN),
            Ready::readable() | UnixReady::error(),
            PollOpt::edge(),
        )?;

        Ok(Interrupt { base, poll, events })
    }

    pub fn trigger(&self) -> Trigger {
        self.base.trigger
    }

    pub fn set_trigger(&mut self, trigger: Trigger) -> Result<()> {
        self.base.set_trigger(trigger)
    }

    pub fn level(&mut self) -> Result<Level> {
        Ok(self.base.level()?)
    }

    pub fn poll(&mut self, timeout: Option<Duration>) -> Result<Level> {
        // Loop until we get the event we're waiting for, or a timeout occurs
        loop {
            self.poll.poll(&mut self.events, timeout)?;

            // No events means a timeout occurred
            if self.events.is_empty() {
                return Err(Error::TimeOut);
            }

            for event in &self.events {
                if event.token() == Token(TOKEN_PIN) && event.readiness().is_readable()
                    && UnixReady::from(event.readiness()).is_error()
                {
                    return Ok(self.base.level()?);
                }
            }
        }
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

            let mut base = InterruptBase::new(pin, trigger)?;
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
                    } else if event.token() == Token(TOKEN_PIN) && event.readiness().is_readable()
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

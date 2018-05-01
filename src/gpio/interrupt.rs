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

// TODO: Doc comments, unexport interrupts from main thread

use std::collections::HashMap;
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
/// Time out.
        TimeOut { description("interrupt polling timed out while waiting for a trigger") }
/// IO error.
        Io(err: io::Error) { description(err.description()) from() }
/// Sysfs error.
        Sysfs(err: sysfs::Error) { description(err.description()) from() }
/// IO error while communicating with the interrupt polling thread.
        SendIo(err: io::Error) { description(err.description()) }
/// Disconnected while sending a control message to the interrupt polling thread.
        SendDisconnected { description("the receiving half of the channel has disconnected") }
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

const TOKEN_RX: usize = 1000;

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
            pin: pin,
            trigger: trigger,
            sysfs_value: sysfs::open_value(pin)?,
        })
    }

    fn level(&mut self) -> Result<Level> {
        let mut buffer = [0; 1];
        self.sysfs_value.read(&mut buffer)?;
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
            Token(0),
            Ready::readable() | UnixReady::error(),
            PollOpt::edge(),
        )?;

        Ok(Interrupt {
            base: base,
            poll: poll,
            events: events,
        })
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
                if event.token() == Token(0) && event.readiness().is_readable()
                    && UnixReady::from(event.readiness()).is_error()
                {
                    return Ok(self.base.level()?);
                }
            }
        }
    }
}

pub struct AsyncInterrupt {
    base: InterruptBase,
    callback: Box<FnMut(Level) + Send + 'static>,
}

impl AsyncInterrupt {
    pub fn new<C>(pin: u8, trigger: Trigger, callback: C) -> Result<AsyncInterrupt>
    where
        C: FnMut(Level) + Send + 'static,
    {
        Ok(AsyncInterrupt {
            base: InterruptBase::new(pin, trigger)?,
            callback: Box::new(callback),
        })
    }

    pub fn callback(&mut self, level: Level) {
        (self.callback)(level);
    }

    pub fn level(&mut self) -> Result<Level> {
        Ok(self.base.level()?)
    }
}

enum ControlMsg {
    Add(u8, AsyncInterrupt),
    Remove(u8),
    Stop,
}

pub struct EventLoop {
    tx: Option<channel::Sender<ControlMsg>>,
}

impl EventLoop {
    pub fn new() -> EventLoop {
        EventLoop { tx: None }
    }

    fn spawn_pollthread(&mut self) -> &channel::Sender<ControlMsg> {
        let (tx, rx) = channel::channel();

        thread::spawn(move || {
            let mut interrupts = HashMap::new();
            let poll = Poll::new().expect("unable to create Poll instance");
            let mut events = Events::with_capacity(1024);

            poll.register(&rx, Token(TOKEN_RX), Ready::readable(), PollOpt::edge())
                .expect("unable to register Receiver");

            loop {
                poll.poll(&mut events, None).expect("unable to poll events");

                for event in &events {
                    if event.token() == Token(TOKEN_RX) {
                        loop {
                            match rx.try_recv() {
                                Ok(ControlMsg::Add(pin, mut interrupt)) => {
                                    interrupt
                                        .base
                                        .level()
                                        .expect("unable to read Interrupt level");
                                    poll.register(
                                        &interrupt.base,
                                        Token(pin as usize),
                                        Ready::readable() | UnixReady::error(),
                                        PollOpt::edge(),
                                    ).expect("unable to register Interrupt");
                                    interrupts.insert(pin as usize, interrupt);
                                }
                                Ok(ControlMsg::Remove(pin)) => {
                                    if let Some(interrupt) = interrupts.get(&(pin as usize)) {
                                        poll.deregister(&interrupt.base)
                                            .expect("unable to deregister Interrupt");
                                    }

                                    interrupts.remove(&(pin as usize));
                                }
                                Ok(ControlMsg::Stop) => {
                                    return;
                                }
                                Err(TryRecvError::Disconnected) => {
                                    return;
                                }
                                Err(TryRecvError::Empty) => {
                                    break;
                                }
                            }
                        }
                    } else if let Some(interrupt) = interrupts.get_mut(&event.token().0) {
                        if event.readiness().is_readable()
                            && UnixReady::from(event.readiness()).is_error()
                        {
                            let interrupt_value = interrupt.base.level().expect("unable to read Interrupt level");

                            interrupt.callback(interrupt_value);
                        }
                    }
                }
            }
        });

        self.tx = Some(tx);
        self.tx.as_ref().unwrap()
    }

    pub fn set_interrupt<C>(&mut self, pin: u8, trigger: Trigger, callback: C) -> Result<()>
    where
        C: FnMut(Level) + Send + 'static,
    {
        // Only spawn a thread for interrupt polling if we're actually using interrupts.
        let tx = if let Some(ref tx) = self.tx {
            tx
        } else {
            self.spawn_pollthread()
        };

        tx.send(ControlMsg::Add(
            pin,
            AsyncInterrupt::new(pin, trigger, callback)?,
        ))?;

        Ok(())
    }

    pub fn clear_interrupt(&self, pin: u8) -> Result<()> {
        if let Some(ref tx) = self.tx {
            tx.send(ControlMsg::Remove(pin))?;
        }

        Ok(())
    }

    pub fn stop(&mut self) -> Result<()> {
        if let Some(ref tx) = self.tx {
            tx.send(ControlMsg::Stop)?;
        }

        Ok(())
    }
}

impl Drop for EventLoop {
    fn drop(&mut self) {
        self.stop().ok();
    }
}

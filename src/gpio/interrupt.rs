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
use std::io::{Read, Seek, SeekFrom};
use std::os::unix::io::AsRawFd;
use std::thread;
use std::time::{Duration, Instant};

use gpio::epoll::{epoll_event, Epoll, EventFd, EPOLLERR, EPOLLET, EPOLLIN, EPOLLPRI};
use gpio::sysfs;
use gpio::{Error, Level, Result, Trigger};

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
        sysfs::set_direction(pin, sysfs::Direction::In)?;
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

    fn fd(&self) -> i32 {
        self.sysfs_value.as_raw_fd()
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

#[derive(Debug)]
struct TriggerStatus {
    interrupt: Option<Interrupt>,
    triggered: bool,
    level: Level,
}

pub struct EventLoop {
    poll: Epoll,
    events: Vec<epoll_event>,
    trigger_status: Vec<TriggerStatus>,
}

impl fmt::Debug for EventLoop {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("EventLoop")
            .field("poll", &self.poll)
            .field("events", &format_args!("{{ .. }}"))
            .field("trigger_status", &format_args!("{{ .. }}"))
            .finish()
    }
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
            poll: Epoll::new()?,
            events: vec![epoll_event { events: 0, u64: 0 }; capacity],
            trigger_status,
        })
    }

    pub fn poll(
        &mut self,
        pins: &[u8],
        reset: bool,
        timeout: Option<Duration>,
    ) -> Result<Option<(u8, Level)>> {
        for pin in pins {
            if *pin as usize >= self.trigger_status.capacity() {
                return Err(Error::InvalidPin(*pin));
            }

            // Did we cache any trigger events during the previous poll?
            if self.trigger_status[*pin as usize].triggered {
                self.trigger_status[*pin as usize].triggered = false;

                if !reset {
                    return Ok(Some((*pin, self.trigger_status[*pin as usize].level)));
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
            let num_events = self.poll.wait(&mut self.events, timeout)?;

            // No events means a timeout occurred
            if num_events == 0 {
                return Ok(None);
            }

            for event in &self.events[0..num_events] {
                let pin = event.u64 as usize;
                if pin < self.trigger_status.capacity() {
                    self.trigger_status[pin].triggered = true;
                    self.trigger_status[pin].level =
                        if let Some(ref mut interrupt) = self.trigger_status[pin].interrupt {
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
                    return Ok(Some((*pin, self.trigger_status[*pin as usize].level)));
                }
            }

            // It's possible a pin we're not waiting for continuously triggers
            // an interrupt, causing repeated loops with calls to poll() using a
            // reset timeout value. Make sure we haven't been looping longer than
            // the requested timeout.
            if let Some(t) = timeout {
                if now.elapsed() > t {
                    return Ok(None);
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
        self.poll
            .add(base.fd(), u64::from(pin), EPOLLERR | EPOLLET | EPOLLPRI)?;
        self.trigger_status[pin as usize].interrupt = Some(base);

        Ok(())
    }

    pub fn clear_interrupt(&mut self, pin: u8) -> Result<()> {
        self.trigger_status[pin as usize].triggered = false;

        if let Some(interrupt) = self.trigger_status[pin as usize].interrupt.take() {
            self.poll.delete(interrupt.fd())?;
        }

        Ok(())
    }
}

pub struct AsyncInterrupt {
    pin: u8,
    poll_thread: Option<thread::JoinHandle<Result<()>>>,
    tx: EventFd,
}

impl AsyncInterrupt {
    pub fn new<C>(pin: u8, trigger: Trigger, mut callback: C) -> Result<AsyncInterrupt>
    where
        C: FnMut(Level) + Send + 'static,
    {
        let tx = EventFd::new()?;
        let rx = tx.fd();

        let poll_thread = thread::spawn(move || -> Result<()> {
            let poll = Epoll::new()?;

            // rx becomes readable when the main thread calls notify()
            poll.add(rx, rx as u64, EPOLLERR | EPOLLET | EPOLLIN)?;

            // Triggering an interrupt sets error and priority
            let mut base = Interrupt::new(pin, trigger)?;
            let base_fd = base.fd();
            base.level()?;
            poll.add(base_fd, base_fd as u64, EPOLLERR | EPOLLET | EPOLLPRI)?;

            let mut events = [epoll_event { events: 0, u64: 0 }; 2];
            loop {
                let num_events = poll.wait(&mut events, None)?;
                if num_events > 0 {
                    for event in &events[0..num_events] {
                        let fd = event.u64 as i32;
                        if fd == rx {
                            return Ok(()); // The main thread asked us to stop
                        } else if fd == base.fd() {
                            callback(base.level()?);
                        }
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
        self.tx.notify()?;

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
            .field("tx", &self.tx)
            .finish()
    }
}

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

#![allow(dead_code)]

use std::fmt;
use std::thread;
use std::time::{Duration, Instant};

use crate::gpio::epoll::{epoll_event, Epoll, EventFd, EPOLLERR, EPOLLET, EPOLLIN, EPOLLPRI};
use crate::gpio::ioctl;
use crate::gpio::pin::InputPin;
use crate::gpio::{Error, Level, Result, Trigger};

#[derive(Debug)]
struct Interrupt {
    pin: u8,
    trigger: Trigger,
    cdev_fd: i32,
    event_request: ioctl::EventRequest,
}

impl Interrupt {
    fn new(cdev_fd: i32, pin: u8, trigger: Trigger) -> Result<Interrupt> {
        Ok(Interrupt {
            pin,
            trigger,
            cdev_fd,
            event_request: ioctl::EventRequest::new(cdev_fd, pin, trigger)?,
        })
    }

    fn trigger(&self) -> Trigger {
        self.trigger
    }

    fn fd(&self) -> i32 {
        self.event_request.fd
    }

    fn pin(&self) -> u8 {
        self.pin
    }

    fn set_trigger(&mut self, trigger: Trigger) -> Result<()> {
        self.trigger = trigger;

        self.reset()
    }

    fn event(&mut self) -> Result<ioctl::Event> {
        // This might block if there are no events waiting
        ioctl::get_event(self.event_request.fd)
    }

    fn reset(&mut self) -> Result<()> {
        // Close the old event fd before opening a new one
        self.event_request.close();
        self.event_request = ioctl::EventRequest::new(self.cdev_fd, self.pin, self.trigger)?;

        Ok(())
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
    cdev_fd: i32,
}

impl fmt::Debug for EventLoop {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventLoop")
            .field("poll", &self.poll)
            .field("events", &format_args!("{{ .. }}"))
            .field("trigger_status", &format_args!("{{ .. }}"))
            .field("cdev_fd", &self.cdev_fd)
            .finish()
    }
}

impl EventLoop {
    pub fn new(cdev_fd: i32, capacity: usize) -> Result<EventLoop> {
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
            cdev_fd,
        })
    }

    pub fn poll<'a>(
        &mut self,
        pins: &[&'a InputPin],
        reset: bool,
        timeout: Option<Duration>,
    ) -> Result<Option<(&'a InputPin, Level)>> {
        for pin in pins {
            let trigger_status = &mut self.trigger_status[pin.pin() as usize];

            // Did we cache any trigger events during the previous poll?
            if trigger_status.triggered {
                trigger_status.triggered = false;

                if !reset {
                    return Ok(Some((pin, trigger_status.level)));
                }
            }

            // Reset any pending trigger events
            if let Some(ref mut interrupt) = trigger_status.interrupt {
                if reset {
                    self.poll.delete(interrupt.fd())?;
                    interrupt.reset()?;
                    self.poll.add(
                        interrupt.fd(),
                        u64::from(interrupt.pin()),
                        EPOLLIN | EPOLLPRI,
                    )?;
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

                let trigger_status = &mut self.trigger_status[pin];

                debug_assert!(
                    trigger_status.interrupt.is_some(),
                    format!("No interrupt set for pin {}", pin)
                );

                if let Some(ref mut interrupt) = trigger_status.interrupt {
                    trigger_status.level = match interrupt.event()?.trigger {
                        Trigger::RisingEdge => Level::High,
                        Trigger::FallingEdge => Level::Low,
                        _ => unsafe { std::hint::unreachable_unchecked() },
                    };

                    trigger_status.triggered = true;
                };
            }

            // Were any interrupts triggered? If so, return one. The rest
            // will be saved for the next poll.
            for pin in pins {
                let trigger_status = &mut self.trigger_status[pin.pin() as usize];

                if trigger_status.triggered {
                    trigger_status.triggered = false;
                    return Ok(Some((pin, trigger_status.level)));
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
        let trigger_status = &mut self.trigger_status[pin as usize];

        trigger_status.triggered = false;

        // Interrupt already exists. We just need to change the trigger.
        if let Some(ref mut interrupt) = trigger_status.interrupt {
            if interrupt.trigger != trigger {
                // This requires a new event request, so the fd might change
                self.poll.delete(interrupt.fd())?;
                interrupt.set_trigger(trigger)?;
                self.poll
                    .add(interrupt.fd(), u64::from(pin), EPOLLIN | EPOLLPRI)?;
            }

            return Ok(());
        }

        // Register a new interrupt
        let interrupt = Interrupt::new(self.cdev_fd, pin, trigger)?;
        self.poll
            .add(interrupt.fd(), u64::from(pin), EPOLLIN | EPOLLPRI)?;
        trigger_status.interrupt = Some(interrupt);

        Ok(())
    }

    pub fn clear_interrupt(&mut self, pin: u8) -> Result<()> {
        let trigger_status = &mut self.trigger_status[pin as usize];

        trigger_status.triggered = false;

        if let Some(interrupt) = trigger_status.interrupt.take() {
            self.poll.delete(interrupt.fd())?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct AsyncInterrupt {
    poll_thread: Option<thread::JoinHandle<Result<()>>>,
    tx: EventFd,
}

impl AsyncInterrupt {
    pub fn new<C>(fd: i32, pin: u8, trigger: Trigger, mut callback: C) -> Result<AsyncInterrupt>
    where
        C: FnMut(Level) + Send + 'static,
    {
        let tx = EventFd::new()?;
        let rx = tx.fd();

        let poll_thread = thread::spawn(move || -> Result<()> {
            let poll = Epoll::new()?;

            // rx becomes readable when the main thread calls notify()
            poll.add(rx, rx as u64, EPOLLERR | EPOLLET | EPOLLIN)?;

            let mut interrupt = Interrupt::new(fd, pin, trigger)?;
            poll.add(interrupt.fd(), interrupt.fd() as u64, EPOLLIN | EPOLLPRI)?;

            let mut events = [epoll_event { events: 0, u64: 0 }; 2];
            loop {
                let num_events = poll.wait(&mut events, None)?;
                if num_events > 0 {
                    for event in &events[0..num_events] {
                        let fd = event.u64 as i32;
                        if fd == rx {
                            return Ok(()); // The main thread asked us to stop
                        } else if fd == interrupt.fd() {
                            let level = match interrupt.event()?.trigger {
                                Trigger::RisingEdge => Level::High,
                                _ => Level::Low,
                            };

                            callback(level);
                        }
                    }
                }
            }
        });

        Ok(AsyncInterrupt {
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
        // Don't wait for the poll thread to exit if the main thread is panicking,
        // because we could potentially block indefinitely while unwinding if the
        // poll thread is executing a callback that doesn't return.
        if !thread::panicking() {
            let _ = self.stop();
        }
    }
}

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

#![cfg_attr(feature = "cargo-clippy", allow(duration_subsec))]

use std::io;
use std::result;
use std::time::Duration;

use libc;

pub use libc::{epoll_event, EPOLLERR, EPOLLET, EPOLLIN, EPOLLONESHOT, EPOLLOUT, EPOLLPRI};

pub type Result<T> = result::Result<T, io::Error>;

fn parse_retval(retval: libc::c_int) -> Result<i32> {
    if retval == -1 {
        Err(io::Error::last_os_error())
    } else {
        Ok(retval)
    }
}

// We're using EventFd to wake up another thread
// that's waiting for epoll_wait() to return.
#[derive(Debug)]
pub struct EventFd {
    fd: i32,
}

impl EventFd {
    pub fn new() -> Result<EventFd> {
        Ok(EventFd {
            fd: unsafe {
                parse_retval(libc::eventfd(0, libc::EFD_NONBLOCK | libc::EFD_SEMAPHORE))?
            },
        })
    }

    pub fn notify(&self) -> Result<()> {
        let buffer: u64 = 1;

        if unsafe { libc::write(self.fd, &buffer as *const u64 as *const libc::c_void, 8) } == -1 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    pub fn fd(&self) -> i32 {
        self.fd
    }
}

impl Drop for EventFd {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.fd);
        }
    }
}

#[derive(Debug)]
pub struct Epoll {
    fd: libc::c_int,
}

impl Epoll {
    pub fn new() -> Result<Epoll> {
        Ok(Epoll {
            fd: unsafe { parse_retval(libc::epoll_create1(0))? },
        })
    }

    pub fn add(&self, fd: i32, id: u64, event_mask: i32) -> Result<()> {
        let mut event = libc::epoll_event {
            events: event_mask as u32,
            u64: id as u64,
        };

        unsafe {
            parse_retval(libc::epoll_ctl(
                self.fd,
                libc::EPOLL_CTL_ADD,
                fd,
                &mut event,
            ))?;
        }

        Ok(())
    }

    pub fn modify(&self, fd: i32, id: u64, event_mask: i32) -> Result<()> {
        let mut event = libc::epoll_event {
            events: event_mask as u32,
            u64: id as u64,
        };

        unsafe {
            parse_retval(libc::epoll_ctl(
                self.fd,
                libc::EPOLL_CTL_MOD,
                fd,
                &mut event,
            ))?;
        }

        Ok(())
    }

    pub fn delete(&self, fd: i32) -> Result<()> {
        let mut event = libc::epoll_event { events: 0, u64: 0 };

        unsafe {
            parse_retval(libc::epoll_ctl(
                self.fd,
                libc::EPOLL_CTL_DEL,
                fd,
                &mut event,
            ))?;
        }

        Ok(())
    }

    pub fn wait(
        &self,
        events: &mut [libc::epoll_event],
        timeout: Option<Duration>,
    ) -> Result<usize> {
        if events.is_empty() {
            return Ok(0);
        }

        let timeout: i32 = if let Some(duration) = timeout {
            (duration.as_secs() * 1_000) as i32 + (duration.subsec_nanos() / 1_000_000) as i32
            // (duration.as_secs() * 1_000) as i32 + duration.subsec_millis() as i32
        } else {
            -1
        };

        unsafe {
            Ok(parse_retval(libc::epoll_wait(
                self.fd,
                events.as_mut_ptr(),
                events.len() as i32,
                timeout,
            ))? as usize)
        }
    }
}

impl Drop for Epoll {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.fd);
        }
    }
}

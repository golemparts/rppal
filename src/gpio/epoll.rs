#![allow(clippy::unnecessary_cast)]
#![allow(dead_code)]

use std::io;
use std::result;
use std::time::Duration;

use libc::{
    self, c_int, c_void, EFD_NONBLOCK, EFD_SEMAPHORE, EPOLL_CTL_ADD, EPOLL_CTL_DEL, EPOLL_CTL_MOD,
};

pub use libc::{epoll_event, EPOLLERR, EPOLLET, EPOLLIN, EPOLLONESHOT, EPOLLOUT, EPOLLPRI};

pub type Result<T> = result::Result<T, io::Error>;

// We're using EventFd to wake up another thread
// that's waiting for epoll_wait() to return.
#[derive(Debug)]
pub struct EventFd {
    fd: i32,
}

impl EventFd {
    pub fn new() -> Result<EventFd> {
        Ok(EventFd {
            fd: parse_retval!(unsafe { libc::eventfd(0, EFD_NONBLOCK | EFD_SEMAPHORE) })?,
        })
    }

    pub fn notify(&self) -> Result<()> {
        let buffer: u64 = 1;

        parse_retval!(unsafe { libc::write(self.fd, &buffer as *const u64 as *const c_void, 8) })?;

        Ok(())
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
    fd: c_int,
}

impl Epoll {
    pub fn new() -> Result<Epoll> {
        Ok(Epoll {
            fd: parse_retval!(unsafe { libc::epoll_create1(0) })?,
        })
    }

    pub fn add(&self, fd: i32, id: u64, event_mask: i32) -> Result<()> {
        let mut event = epoll_event {
            events: event_mask as u32,
            u64: id as u64,
        };

        parse_retval!(unsafe { libc::epoll_ctl(self.fd, EPOLL_CTL_ADD, fd, &mut event) })?;

        Ok(())
    }

    pub fn modify(&self, fd: i32, id: u64, event_mask: i32) -> Result<()> {
        let mut event = epoll_event {
            events: event_mask as u32,
            u64: id as u64,
        };

        parse_retval!(unsafe { libc::epoll_ctl(self.fd, EPOLL_CTL_MOD, fd, &mut event) })?;

        Ok(())
    }

    pub fn delete(&self, fd: i32) -> Result<()> {
        let mut event = epoll_event { events: 0, u64: 0 };

        parse_retval!(unsafe { libc::epoll_ctl(self.fd, EPOLL_CTL_DEL, fd, &mut event) })?;

        Ok(())
    }

    pub fn wait(&self, events: &mut [epoll_event], timeout: Option<Duration>) -> Result<usize> {
        if events.is_empty() {
            return Ok(0);
        }

        let timeout = if let Some(duration) = timeout {
            (duration.as_secs() * 1_000 + u64::from(duration.subsec_millis())) as c_int
        } else {
            -1
        };

        Ok(parse_retval!(unsafe {
            libc::epoll_wait(self.fd, events.as_mut_ptr(), events.len() as c_int, timeout)
        })? as usize)
    }
}

impl Drop for Epoll {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.fd);
        }
    }
}

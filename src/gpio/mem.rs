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

#![allow(dead_code)]

use std::fs::OpenOptions;
use std::io;
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::io::AsRawFd;
use std::ptr;

use libc;

use gpio::{Error, Result};
use system::DeviceInfo;

// The BCM2835 has 41 32-bit registers related to the GPIO (datasheet @ 6.1).
const GPIO_MEM_SIZE: usize = 164;

#[derive(Debug)]
pub struct GpioMem {
    mapped: bool,
    mem_ptr: *mut u32,
}

impl GpioMem {
    pub fn new() -> GpioMem {
        GpioMem {
            mapped: false,
            mem_ptr: ptr::null_mut(),
        }
    }

    pub fn open(&mut self) -> Result<()> {
        if self.mapped {
            return Ok(());
        }

        // Try /dev/gpiomem first. If that fails, try /dev/mem instead. If neither works,
        // report back the error that's the most relevant.
        self.mem_ptr = match self.map_devgpiomem() {
            Ok(ptr) => ptr,
            Err(gpiomem_err) => match self.map_devmem() {
                Ok(ptr) => ptr,
                Err(Error::Io(ref e)) if e.kind() == io::ErrorKind::PermissionDenied => {
                    return Err(Error::PermissionDenied)
                }
                Err(Error::UnknownSoC) => return Err(Error::UnknownSoC),
                _ => return Err(gpiomem_err),
            },
        };

        self.mapped = true;

        Ok(())
    }

    fn map_devgpiomem(&self) -> Result<*mut u32> {
        // Open /dev/gpiomem with read/write/sync flags. This might fail if
        // /dev/gpiomem doesn't exist (< Raspbian Jessie), or /dev/gpiomem
        // doesn't have the appropriate permissions, or the current user is
        // not a member of the gpio group.
        let gpiomem_file = match OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_SYNC)
            .open("/dev/gpiomem")
        {
            Ok(file) => file,
            Err(e) => return Err(Error::Io(e)),
        };

        // Memory-map /dev/gpiomem at offset 0
        let gpiomem_ptr = unsafe {
            libc::mmap(
                ptr::null_mut(),
                GPIO_MEM_SIZE,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                gpiomem_file.as_raw_fd(),
                0,
            )
        };

        if gpiomem_ptr == libc::MAP_FAILED {
            return Err(Error::Io(io::Error::last_os_error()));
        }

        Ok(gpiomem_ptr as *mut u32)
    }

    fn map_devmem(&self) -> Result<*mut u32> {
        // Identify which SoC we're using, so we know what offset to start at
        let device_info = match DeviceInfo::new() {
            Ok(s) => s,
            Err(_) => return Err(Error::UnknownSoC),
        };

        let mem_file = match OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_SYNC)
            .open("/dev/mem")
        {
            Ok(file) => file,
            Err(e) => return Err(Error::Io(e)),
        };

        // Memory-map /dev/mem at the appropriate offset for our SoC
        let mem_ptr = unsafe {
            libc::mmap(
                ptr::null_mut(),
                GPIO_MEM_SIZE,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                mem_file.as_raw_fd(),
                (device_info.peripheral_base() + device_info.gpio_offset()) as libc::off_t,
            )
        };

        if mem_ptr == libc::MAP_FAILED {
            return Err(Error::Io(io::Error::last_os_error()));
        }

        Ok(mem_ptr as *mut u32)
    }

    pub fn close(&mut self) {
        if !self.mapped {
            return;
        }

        unsafe {
            libc::munmap(
                self.mem_ptr as *mut libc::c_void,
                GPIO_MEM_SIZE as libc::size_t,
            );
        }

        self.mapped = false;
    }

    pub fn read(&self, offset: usize) -> u32 {
        if !self.mapped || offset >= GPIO_MEM_SIZE {
            return 0;
        }

        unsafe { ptr::read_volatile(self.mem_ptr.offset(offset as isize)) }
    }

    pub fn write(&self, offset: usize, value: u32) {
        if !self.mapped || offset >= GPIO_MEM_SIZE {
            return;
        }

        unsafe {
            ptr::write_volatile(self.mem_ptr.offset(offset as isize), value);
        }
    }
}

impl Drop for GpioMem {
    fn drop(&mut self) {
        self.close();
    }
}

// Required because of the raw pointer to our memory-mapped file
unsafe impl Send for GpioMem {}

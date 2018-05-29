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

use libc::{c_int, c_ulong, ioctl};
use std::fmt;
use std::marker;
use std::mem::size_of;

use super::*;

const TYPE_SPI: c_ulong = (b'k' as c_ulong) << TYPESHIFT;

const NR_MESSAGE: c_ulong = 0 << NRSHIFT;
const NR_MODE: c_ulong = 1 << NRSHIFT;
const NR_LSB_FIRST: c_ulong = 2 << NRSHIFT;
const NR_BITS_PER_WORD: c_ulong = 3 << NRSHIFT;
const NR_MAX_SPEED_HZ: c_ulong = 4 << NRSHIFT;
const NR_MODE32: c_ulong = 5 << NRSHIFT;

const REQ_RD_MODE: c_ulong = (DIR_READ | TYPE_SPI | NR_MODE | SIZE_U8);
const REQ_RD_LSB_FIRST: c_ulong = (DIR_READ | TYPE_SPI | NR_LSB_FIRST | SIZE_U8);
const REQ_RD_BITS_PER_WORD: c_ulong = (DIR_READ | TYPE_SPI | NR_BITS_PER_WORD | SIZE_U8);
const REQ_RD_MAX_SPEED_HZ: c_ulong = (DIR_READ | TYPE_SPI | NR_MAX_SPEED_HZ | SIZE_U32);
const REQ_RD_MODE_32: c_ulong = (DIR_READ | TYPE_SPI | NR_MODE32 | SIZE_U32);

const REQ_WR_MESSAGE: c_ulong = (DIR_WRITE | TYPE_SPI | NR_MESSAGE);
const REQ_WR_MODE: c_ulong = (DIR_WRITE | TYPE_SPI | NR_MODE | SIZE_U8);
const REQ_WR_LSB_FIRST: c_ulong = (DIR_WRITE | TYPE_SPI | NR_LSB_FIRST | SIZE_U8);
const REQ_WR_BITS_PER_WORD: c_ulong = (DIR_WRITE | TYPE_SPI | NR_BITS_PER_WORD | SIZE_U8);
const REQ_WR_MAX_SPEED_HZ: c_ulong = (DIR_WRITE | TYPE_SPI | NR_MAX_SPEED_HZ | SIZE_U32);
const REQ_WR_MODE_32: c_ulong = (DIR_WRITE | TYPE_SPI | NR_MODE32 | SIZE_U32);

pub const MODE_CPHA: u8 = 0x01;
pub const MODE_CPOL: u8 = 0x02;

pub const MODE_0: u8 = 0;
pub const MODE_1: u8 = MODE_CPHA;
pub const MODE_2: u8 = MODE_CPOL;
pub const MODE_3: u8 = MODE_CPOL | MODE_CPHA;

pub const MODE_CS_HIGH: u8 = 0x04; // Set SS to active high
pub const MODE_LSB_FIRST: u8 = 0x08; // Set bit order to LSB first
pub const MODE_3WIRE: u8 = 0x10; // Set bidirectional mode
pub const MODE_LOOP: u8 = 0x20; // Set loopback mode
pub const MODE_NO_CS: u8 = 0x40; // Don't assert SS
pub const MODE_READY: u8 = 0x80; // Slave sends a ready signal
pub const MODE_TX_DUAL: u32 = 0x100; // Send on 2 outgoing lines
pub const MODE_TX_QUAD: u32 = 0x200; // Send on 4 outgoing lines
pub const MODE_RX_DUAL: u32 = 0x400; // Receive on 2 incoming lines
pub const MODE_RX_QUAD: u32 = 0x800; // Receive on 4 incoming lines

/// Part of a multi-segment transfer
#[derive(PartialEq, Copy, Clone)]
#[repr(C)]
pub struct TransferSegment<'a, 'b> {
    // Pointer to write buffer, or 0.
    tx_buf: u64,
    // Pointer to read buffer, or 0.
    rx_buf: u64,
    // Number of bytes to transfer in this segment.
    len: u32,
    // Set a different clock speed for this segment. Default = 0.
    speed_hz: u32,
    // Add a delay before the (optional) SS change and the next segment.
    delay_usecs: u16,
    // Bits per word for this segment. The Pi only supports 8 bits (or 9 bits in LoSSI mode). Default = 0.
    bits_per_word: u8,
    // Set to 1 to briefly set SS inactive between this segment and the next. If this is the last segment, keep SS active.
    cs_change: u8,
    // Number of outgoing lines used for dual/quad SPI. Not supported on the Raspberry Pi. Default = 0.
    tx_nbits: u8,
    // Number of incoming lines used for dual/quad SPI. Not supported on the Raspberry Pi. Default = 0.
    rx_nbits: u8,
    // Padding. Set to 0 for forward compatibility.
    pad: u16,
    // Zero-sized variable used to link this struct to the read buffer lifetime.
    read_buffer_lifetime: marker::PhantomData<&'a mut [u8]>,
    // Zero-sized variable used to link this struct to the write buffer lifetime.
    write_buffer_lifetime: marker::PhantomData<&'b [u8]>,
}

impl<'a, 'b> TransferSegment<'a, 'b> {
    /// Creates a new `TransferSegment` with the default settings.
    ///
    /// If `read_buffer` is set to `None`, any incoming data is discarded.
    ///
    /// If `write_buffer` is set to `None`, a zero-value byte will be sent for every
    /// byte read.
    ///
    /// If both `read_buffer` and `write_buffer` are specified, [`transfer_segments`]
    /// will only transfer as many bytes as the shortest of the two buffers contains.
    ///
    /// By default, all customizable settings are set to 0, which means it uses
    /// the same values as set for [`Spi`].
    ///
    /// [`transfer_segments`]: index.html
    /// [`Spi`]: index.html
    pub fn new(
        read_buffer: Option<&'a mut [u8]>,
        write_buffer: Option<&'b [u8]>,
    ) -> TransferSegment<'a, 'b> {
        TransferSegment::with_settings(read_buffer, write_buffer, 0, 0, 0, false)
    }

    /// Creates a new `TransferSegment` with the specified settings.
    ///
    /// These settings override the values set for [`Spi`], and are only used
    /// for this specific segment.
    ///
    /// If `read_buffer` is set to `None`, any incoming data is discarded.
    ///
    /// If `write_buffer` is set to `None`, a zero-value byte will be sent for every
    /// byte read.
    ///
    /// If both `read_buffer` and `write_buffer` are specified, [`transfer_segments`]
    /// will only transfer as many bytes as the shortest of the two buffers contains.
    ///
    /// `clock_speed` sets an alternate clock speed in hertz (Hz).
    ///
    /// `delay` sets a delay in microseconds (µs).
    ///
    /// `bits_per_word` sets the number of bits per word. The Raspberry Pi currently only supports 8 bits per word.
    ///
    /// `ss_change` changes how Slave Select behaves in between two segments (toggle SS), or after the final segment (keep SS active).
    ///
    /// [`transfer_segments`]: index.html
    /// [`Spi`]: index.html
    pub fn with_settings(
        read_buffer: Option<&'a mut [u8]>,
        write_buffer: Option<&'b [u8]>,
        clock_speed: u32,
        delay: u16,
        bits_per_word: u8,
        ss_change: bool,
    ) -> TransferSegment<'a, 'b> {
        // Len will contain the length of the shortest of the supplied buffers
        let mut len: u32 = 0;

        let tx_buf = if let Some(buffer) = write_buffer {
            len = buffer.len() as u32;
            buffer.as_ptr() as u64
        } else {
            0
        };

        let rx_buf = if let Some(buffer) = read_buffer {
            if len > buffer.len() as u32 {
                len = buffer.len() as u32;
            }
            buffer.as_ptr() as u64
        } else {
            0
        };

        TransferSegment {
            tx_buf,
            rx_buf,
            len,
            speed_hz: clock_speed,
            delay_usecs: delay,
            bits_per_word,
            cs_change: ss_change as u8,
            tx_nbits: 0,
            rx_nbits: 0,
            pad: 0,
            read_buffer_lifetime: marker::PhantomData,
            write_buffer_lifetime: marker::PhantomData,
        }
    }

    /// Returns the number of bytes that will be transferred.
    ///
    /// If both a read buffer and write buffer are supplied, [`transfer_segments`] only
    /// transfers as many bytes as the shortest of the two buffers contains.
    ///
    /// [`transfer_segments`]: struct.Spi.html#method.transfer_segments
    pub fn len(&self) -> usize {
        self.len as usize
    }

    /// Returns `true` if this segment won't transfer any bytes.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Gets the alternate clock speed in hertz (Hz) for this segment.
    pub fn clock_speed(&self) -> u32 {
        self.speed_hz
    }

    /// Sets an alternate clock speed in hertz (Hz) for this segment.
    ///
    /// By default, `clock_speed` is set to 0, which means
    /// it will use the same value as configured for `Spi`.
    pub fn set_clock_speed(&mut self, clock_speed: u32) {
        self.speed_hz = clock_speed;
    }

    /// Gets the delay in microseconds (µs) for this segment.
    pub fn delay(&self) -> u16 {
        self.delay_usecs
    }

    /// Sets a delay in microseconds (µs) for this segment.
    ///
    /// `set_delay` adds a delay at the end of this segment,
    /// before the (optional) Slave Select change.
    ///
    /// By default, `delay` is set to 0.
    pub fn set_delay(&mut self, delay: u16) {
        self.delay_usecs = delay;
    }

    /// Gets the number of bits per word for this segment.
    pub fn bits_per_word(&self) -> u8 {
        self.bits_per_word
    }

    /// Sets the number of bits per word for this segment.
    ///
    /// The Raspberry Pi currently only supports 8 bit words.
    ///
    /// By default, `bits_per_word` is set to 0, which means
    /// it will use the same value as configured for `Spi`.
    pub fn set_bits_per_word(&mut self, bits_per_word: u8) {
        self.bits_per_word = bits_per_word;
    }

    /// Gets the state of Slave Select change for this segment.
    pub fn ss_change(&self) -> bool {
        self.cs_change == 1
    }

    /// Sets alternate Slave Select behavior for this segment.
    ///
    /// If `ss_change` is set to `true`, and this is not the last
    /// segment of the transfer, the Slave Select line will briefly
    /// change to inactive between this segment and the next.
    /// If this is the last segment, setting `ss_change` to true will
    /// keep Slave Select active after the transfer ends.
    ///
    /// By default, `ss_change` is set to `false`.
    pub fn set_ss_change(&mut self, ss_change: bool) {
        self.cs_change = ss_change as u8;
    }
}

impl<'a, 'b> fmt::Debug for TransferSegment<'a, 'b> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("TransferSegment")
            .field("tx_buf", &self.tx_buf)
            .field("rx_buf", &self.rx_buf)
            .field("len", &self.len)
            .field("speed_hz", &self.speed_hz)
            .field("delay_usecs", &self.delay_usecs)
            .field("bits_per_word", &self.bits_per_word)
            .field("cs_change", &self.cs_change)
            .field("tx_nbits", &self.tx_nbits)
            .field("rx_nbits", &self.rx_nbits)
            .field("pad", &self.pad)
            .finish()
    }
}

pub unsafe fn mode(fd: c_int, value: &mut u8) -> Result<i32> {
    parse_retval(ioctl(fd, REQ_RD_MODE, value))
}

pub unsafe fn set_mode(fd: c_int, value: u8) -> Result<i32> {
    parse_retval(ioctl(fd, REQ_WR_MODE, &value))
}

pub unsafe fn lsb_first(fd: c_int, value: &mut u8) -> Result<i32> {
    parse_retval(ioctl(fd, REQ_RD_LSB_FIRST, value))
}

pub unsafe fn set_lsb_first(fd: c_int, value: u8) -> Result<i32> {
    parse_retval(ioctl(fd, REQ_WR_LSB_FIRST, &value))
}

pub unsafe fn bits_per_word(fd: c_int, value: &mut u8) -> Result<i32> {
    parse_retval(ioctl(fd, REQ_RD_BITS_PER_WORD, value))
}

pub unsafe fn set_bits_per_word(fd: c_int, value: u8) -> Result<i32> {
    parse_retval(ioctl(fd, REQ_WR_BITS_PER_WORD, &value))
}

pub unsafe fn clock_speed(fd: c_int, value: &mut u32) -> Result<i32> {
    parse_retval(ioctl(fd, REQ_RD_MAX_SPEED_HZ, value))
}

pub unsafe fn set_clock_speed(fd: c_int, value: u32) -> Result<i32> {
    parse_retval(ioctl(fd, REQ_WR_MAX_SPEED_HZ, &value))
}

pub unsafe fn mode32(fd: c_int, value: &mut u32) -> Result<i32> {
    parse_retval(ioctl(fd, REQ_RD_MODE_32, value))
}

pub unsafe fn set_mode32(fd: c_int, value: u32) -> Result<i32> {
    parse_retval(ioctl(fd, REQ_WR_MODE_32, &value))
}

pub unsafe fn transfer(fd: c_int, segments: &[TransferSegment]) -> Result<i32> {
    parse_retval(ioctl(
        fd,
        REQ_WR_MESSAGE
            | (((segments.len() * size_of::<TransferSegment>()) as c_ulong) << SIZESHIFT),
        segments,
    ))
}

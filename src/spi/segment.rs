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

use std::fmt;
use std::marker;

/// Part of a multi-segment transfer.
///
/// `Segment`s are transferred using the [`Spi::transfer_segments`] method.
///
/// Construct a new `Segment` for a simultaneous (full-duplex) read/write
/// transfer using [`new`]. For read operations without any outgoing data,
/// use [`with_read`]. For write operations where any incoming data
/// should be discarded, use [`with_write`].
///
/// [`Spi::transfer_segments`]: struct.Spi.html#method.transfer_segments
/// [`with_read`]: #method.with_read
/// [`with_write`]: #method.with_write
/// [`new`]: #method.new
#[derive(PartialEq, Eq, Copy, Clone)]
#[repr(C)]
pub struct Segment<'a, 'b> {
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

impl<'a, 'b> Segment<'a, 'b> {
    /// Constructs a new `Segment` with the default settings, and configures it
    /// for a simultaneous (full-duplex) read/write transfer.
    ///
    /// For `Segment`s that only require either a read or write operation, call
    /// [`with_read`] or [`with_write`] instead of `new`.
    ///
    /// [`Spi::transfer_segments`] will only transfer as many bytes as the shortest of
    /// the two buffers contains.
    ///
    /// By default, all customizable settings are set to 0, which means it uses
    /// the same values as set for [`Spi`].
    ///
    /// [`Spi::transfer_segments`]: struct.Spi.html#method.transfer_segments
    /// [`Spi`]: struct.Spi.html
    /// [`with_read`]: #method.with_read
    /// [`with_write`]: #method.with_write
    pub fn new(read_buffer: &'a mut [u8], write_buffer: &'b [u8]) -> Segment<'a, 'b> {
        Segment::with_settings(Some(read_buffer), Some(write_buffer), 0, 0, 0, false)
    }

    /// Constructs a new `Segment` with the default settings, and configures it
    /// for a read operation.
    ///
    /// Incoming data from the slave device is written to `buffer`. The total
    /// number of bytes read depends on the length of `buffer`. A zero-value
    /// byte is sent for every byte read.
    ///
    /// By default, all customizable settings are set to 0, which means it uses
    /// the same values as set for [`Spi`].
    ///
    /// [`Spi`]: struct.Spi.html
    pub fn with_read(buffer: &mut [u8]) -> Segment<'_, '_> {
        Segment::with_settings(Some(buffer), None, 0, 0, 0, false)
    }

    /// Constructs a new `Segment` with the default settings, and configures it
    /// for a write operation.
    ///
    /// Outgoing data from `buffer` is sent to the slave device. Any
    /// incoming data is discarded.
    ///
    /// By default, all customizable settings are set to 0, which means it uses
    /// the same values as set for [`Spi`].
    ///
    /// [`Spi`]: struct.Spi.html
    pub fn with_write(buffer: &[u8]) -> Segment<'_, '_> {
        Segment::with_settings(None, Some(buffer), 0, 0, 0, false)
    }

    /// Constructs a new `Segment` with the specified settings.
    ///
    /// These settings override the values set for [`Spi`], and are only used
    /// for this specific segment.
    ///
    /// If `read_buffer` is set to `None`, any incoming data is discarded.
    ///
    /// If `write_buffer` is set to `None`, a zero-value byte is sent for every
    /// byte read.
    ///
    /// If both `read_buffer` and `write_buffer` are specified, [`Spi::transfer_segments`]
    /// will only transfer as many bytes as the shortest of the two buffers contains.
    ///
    /// `clock_speed` sets a custom clock speed in hertz (Hz).
    ///
    /// `delay` sets a delay in microseconds (µs).
    ///
    /// `bits_per_word` sets the number of bits per word. The Raspberry Pi currently only supports 8 bits per word.
    ///
    /// `ss_change` changes how Slave Select behaves in between two segments (toggle SS), or after the final segment (keep SS active).
    ///
    /// [`Spi::transfer_segments`]: struct.Spi.html#method.transfer_segments
    /// [`Spi`]: struct.Spi.html
    pub fn with_settings(
        read_buffer: Option<&'a mut [u8]>,
        write_buffer: Option<&'b [u8]>,
        clock_speed: u32,
        delay: u16,
        bits_per_word: u8,
        ss_change: bool,
    ) -> Segment<'a, 'b> {
        // Len will contain the length of the shortest of the supplied buffers
        let mut len: u32 = 0;

        let tx_buf = if let Some(buffer) = write_buffer {
            len = buffer.len() as u32;
            buffer.as_ptr() as u64
        } else {
            0
        };

        let rx_buf = if let Some(buffer) = read_buffer {
            if (len > buffer.len() as u32) || tx_buf == 0 {
                len = buffer.len() as u32;
            }
            buffer.as_ptr() as u64
        } else {
            0
        };

        Segment {
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
    /// If both a read buffer and write buffer are supplied,
    /// [`Spi::transfer_segments`] only transfers as many bytes as the
    /// shortest of the two buffers contains.
    ///
    /// [`Spi::transfer_segments`]: struct.Spi.html#method.transfer_segments
    pub fn len(&self) -> usize {
        self.len as usize
    }

    /// Returns `true` if this segment won't transfer any bytes.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Gets the custom clock speed in hertz (Hz) for this segment.
    pub fn clock_speed(&self) -> u32 {
        self.speed_hz
    }

    /// Sets a custom clock speed in hertz (Hz) for this segment.
    ///
    /// The SPI driver will automatically select the closest valid frequency.
    ///
    /// By default, `clock_speed` is set to `0`, which means
    /// it will use the same value as configured for [`Spi`].
    ///
    /// [`Spi`]: struct.Spi.html
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
    /// By default, `delay` is set to `0`.
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
    /// By default, `bits_per_word` is set to `0`, which means
    /// it will use the same value as configured for [`Spi`].
    ///
    /// [`Spi`]: struct.Spi.html
    pub fn set_bits_per_word(&mut self, bits_per_word: u8) {
        self.bits_per_word = bits_per_word;
    }

    /// Gets the state of Slave Select change for this segment.
    pub fn ss_change(&self) -> bool {
        self.cs_change == 1
    }

    /// Changes Slave Select's behavior for this segment.
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

impl<'a, 'b> fmt::Debug for Segment<'a, 'b> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Segment")
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

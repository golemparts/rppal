// Copyright (c) 2017-2021 Rene van der Meer
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

macro_rules! parse_retval {
    ($retval:expr) => {{
        let retval = $retval;

        if retval == -1 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(retval)
        }
    }};
}

// Initialize an array with a type that doesn't implement Copy
macro_rules! init_array {
    ($val:expr, $len:expr) => {{
        // Based on https://doc.rust-lang.org/stable/std/mem/union.MaybeUninit.html#initializing-an-array-element-by-element

        use std::mem::{self, MaybeUninit};

        // Create an uninitialized array of `MaybeUninit`. The `assume_init` is
        // safe because the type we are claiming to have initialized here is a
        // bunch of `MaybeUninit`s, which do not require initialization.
        let mut data: [MaybeUninit<_>; $len] = unsafe { MaybeUninit::uninit().assume_init() };

        // Dropping a `MaybeUninit` does nothing. Thus using raw pointer
        // assignment instead of `ptr::write` does not cause the old
        // uninitialized value to be dropped. Also if there is a panic during
        // this loop, we have a memory leak, but there is no memory safety
        // issue.
        for elem in &mut data[..] {
            *elem = MaybeUninit::new($val);
        }

        // Everything is initialized. Transmute the array to the
        // initialized type.
        unsafe { mem::transmute::<_, [_; $len]>(data) }
    }};
}

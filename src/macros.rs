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

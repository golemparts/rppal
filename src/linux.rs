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

// Find user ID for specified user
#[cfg(linux)]
pub fn user_to_uid(name: &str) -> Option<u32> {
    use std::ffi::CString;
    use std::ptr;
    use libc;

    if let Ok(name_cstr) = CString::new(name) {
        unsafe {
            let mut buf = &mut [0u8; 4096];
            let mut res: *mut libc::passwd = ptr::null_mut();
            let mut pwd = libc::passwd {
                pw_name: ptr::null_mut(),
                pw_passwd: ptr::null_mut(),
                pw_uid: 0,
                pw_gid: 0,
                pw_gecos: ptr::null_mut(),
                pw_dir: ptr::null_mut(),
                pw_shell: ptr::null_mut(),
            };

            if libc::getpwnam_r(
                name_cstr.as_ptr(),
                &mut pwd,
                buf.as_mut_ptr(),
                buf.len(),
                &mut res,
            ) == 0 && res as usize > 0
            {
                return Some((*res).pw_uid);
            }
        }
    }

    None
}

#[cfg(not(linux))]
pub fn user_to_uid(_name: &str) -> Option<u32> {
    None
}

// Find group ID for specified group
#[cfg(linux)]
pub fn group_to_gid(name: &str) -> Option<u32> {
    use std::ffi::CString;
    use std::ptr;
    use libc;

    if let Ok(name_cstr) = CString::new(name) {
        unsafe {
            let mut buf = &mut [0u8; 4096];
            let mut res: *mut libc::group = ptr::null_mut();
            let mut grp = libc::group {
                gr_name: ptr::null_mut(),
                gr_passwd: ptr::null_mut(),
                gr_gid: 0,
                gr_mem: ptr::null_mut(),
            };

            if libc::getgrnam_r(
                name_cstr.as_ptr(),
                &mut grp,
                buf.as_mut_ptr(),
                buf.len(),
                &mut res,
            ) == 0 && res as usize > 0
            {
                return Some((*res).gr_gid);
            }
        }
    }

    None
}

#[cfg(not(linux))]
pub fn group_to_gid(_name: &str) -> Option<u32> {
    None
}

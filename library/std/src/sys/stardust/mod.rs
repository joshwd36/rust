#![allow(missing_docs, nonstandard_style)]

pub mod alloc;
#[path = "../unsupported/args.rs"]
pub mod args;
pub mod cmath;
pub mod condvar;
pub mod env;
pub mod ext;
pub mod fs;
pub mod io;
pub mod memchr;
pub mod mutex;
#[path = "../unsupported/net.rs"]
pub mod net;
pub mod os;
pub mod path;
#[path = "../unsupported/pipe.rs"]
pub mod pipe;
#[path = "../unsupported/process.rs"]
pub mod process;
pub mod rwlock;
pub mod stack_overflow;
pub mod stdio;
pub mod thread;
pub mod thread_local_key;
pub mod time;

pub use crate::sys_common::os_str_bytes as os_str;

use crate::io::ErrorKind;

pub fn unsupported<T>() -> crate::io::Result<T> {
    Err(unsupported_err())
}

pub fn unsupported_err() -> crate::io::Error {
    crate::io::Error::new(crate::io::ErrorKind::Other, "operation not supported on Stardust yet")
}

// This enum is used as the storage for a bunch of types which can't actually
// exist.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub enum Void {}

pub fn abort_internal() -> ! {
    unsafe { libc::abort() }
}

pub fn init() {}

pub fn hashmap_random_keys() -> (u64, u64) {
    let (a, b, c, d) = unsafe {
        (libc::rand() as u64, libc::rand() as u64, libc::rand() as u64, libc::rand() as u64)
    };
    (a << 32 + b, c << 32 + d)
}

pub unsafe fn strlen(mut s: *const libc::c_char) -> usize {
    // SAFETY: The caller must guarantee `s` points to a valid 0-terminated string.
    let mut n = 0;
    while *s != 0 {
        n += 1;
        s = s.offset(1);
    }
    n
}

pub fn decode_error_kind(errno: i32) -> ErrorKind {
    match errno {
        0 => ErrorKind::WriteZero,
        4 | 5 => ErrorKind::NotFound,
        6 | 9 | 11 | 19 => ErrorKind::InvalidInput,
        7 | 8 | 10 => ErrorKind::PermissionDenied,
        14 => ErrorKind::Interrupted,
        15 => ErrorKind::TimedOut,
        _ => ErrorKind::Other,
    }
}

pub fn cvt_nz(error: libc::c_int) -> crate::io::Result<()> {
    if error == 0 { Ok(()) } else { Err(crate::io::Error::from_raw_os_error(error)) }
}

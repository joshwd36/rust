use crate::error::Error as StdError;
use crate::ffi::{CStr, CString, OsStr, OsString};
use crate::fmt;
use crate::io;
use crate::iter;
use crate::os::stardust::ffi::OsStringExt;
use crate::path::{self, PathBuf};
use crate::slice;
use crate::str;
use crate::sys::{unsupported, Void};
use crate::sys_common::os_str_bytes::OsStrExt;

const PATH_SEPARATOR: u8 = b'/';

pub fn errno() -> i32 {
    unsafe { *libc::__errno_location() }
}

pub fn error_string(errno: i32) -> String {
    match errno {
        1 => "Operation not permitted",
        2 => "No such file or directory",
        3 => "No such process",
        4 => "Interrupted system call",
        5 => "I/O error",
        7 => "Arg list too long",
        8 => "Exec format error",
        9 => "Bad file number",
        10 => "No child processes",
        11 => "Try again",
        12 => "Out of memory",
        13 => "Permission denied",
        14 => "Bad address",
        16 => "Device or resource busy",
        17 => "File exists",
        21 => "Is a directory",
        22 => "Invalid argument",
        23 => "File table overflow",
        24 => "Too many open files",
        25 => "Not a typewriter",
        27 => "File too large",
        28 => "No space left on device",
        30 => "Read-only file system",
        32 => "Broken pipe",
        33 => "Math argument out of domain of func",
        34 => "Math result not representable",
        35 => "Resource deadlock would occur",
        36 => "File name too long",
        38 => "Function not implemented",
        39 => "Directory not empty",
        75 => "Value too large for defined data type",
        84 => "Illegal byte sequence",
        88 => "Socket operation on non-socket",
        89 => "Destination address required",
        90 => "Message too long",
        93 => "Protocol not supported",
        95 => "Operation not supported on transport endpoint",
        97 => "Address family not supported by protocol",
        98 => "Address already in use",
        99 => "Cannot assign requested address",
        101 => "Network is unreachable",
        104 => "Connection reset by peer",
        105 => "No buffer space available",
        106 => "Transport endpoint is already connected",
        107 => "Transport endpoint is not connected",
        110 => "Connection timed out",
        111 => "Connection refused",
        113 => "No route to host",
        114 => "Operation already in progress",
        115 => "Operation now in progress",
        122 => "Quota exceeded",
        _ => "Unknown error",
    }
    .to_string()
}

pub fn getcwd() -> io::Result<PathBuf> {
    let mut buf = Vec::with_capacity(512);
    loop {
        unsafe {
            let ptr = buf.as_mut_ptr() as *mut libc::c_char;
            let result = libc::f_getcwd(ptr, buf.capacity() as u32);
            if result == libc::FRESULT_FR_OK {
                let len = CStr::from_ptr(buf.as_ptr() as *const libc::c_char).to_bytes().len();
                buf.set_len(len);
                buf.shrink_to_fit();
                return Ok(PathBuf::from(OsString::from_vec(buf)));
            } else if result != libc::FRESULT_FR_NOT_ENOUGH_CORE {
                return Err(io::Error::from_raw_os_error(result as i32));
            }

            // Trigger the internal buffer resizing logic of `Vec` by requiring
            // more space than the current capacity.
            let cap = buf.capacity();
            buf.set_len(cap);
            buf.reserve(1);
        }
    }
}

pub fn chdir(path: &path::Path) -> io::Result<()> {
    let path = CString::new(path.as_os_str().as_bytes())?;
    let result = unsafe { libc::f_chdir(path.as_ptr()) };
    if result != libc::FRESULT_FR_OK {
        return Err(io::Error::from_raw_os_error(result as i32));
    }
    Ok(())
}

pub struct SplitPaths<'a> {
    iter: iter::Map<slice::Split<'a, u8, fn(&u8) -> bool>, fn(&'a [u8]) -> PathBuf>,
}

pub fn split_paths(unparsed: &OsStr) -> SplitPaths<'_> {
    fn bytes_to_path(b: &[u8]) -> PathBuf {
        PathBuf::from(<OsStr as OsStrExt>::from_bytes(b))
    }
    fn is_separator(b: &u8) -> bool {
        *b == PATH_SEPARATOR
    }
    let unparsed = unparsed.as_bytes();
    SplitPaths {
        iter: unparsed
            .split(is_separator as fn(&u8) -> bool)
            .map(bytes_to_path as fn(&[u8]) -> PathBuf),
    }
}

impl<'a> Iterator for SplitPaths<'a> {
    type Item = PathBuf;
    fn next(&mut self) -> Option<PathBuf> {
        self.iter.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

#[derive(Debug)]
pub struct JoinPathsError;

pub fn join_paths<I, T>(paths: I) -> Result<OsString, JoinPathsError>
    where
        I: Iterator<Item = T>,
        T: AsRef<OsStr>,
{
    let mut joined = Vec::new();

    for (i, path) in paths.enumerate() {
        let path = path.as_ref().as_bytes();
        if i > 0 {
            joined.push(PATH_SEPARATOR)
        }
        if path.contains(&PATH_SEPARATOR) {
            return Err(JoinPathsError);
        }
        joined.extend_from_slice(path);
    }
    Ok(OsStringExt::from_vec(joined))
}

impl fmt::Display for JoinPathsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "path segment contains separator `{}`", PATH_SEPARATOR)
    }
}

impl StdError for JoinPathsError {
    #[allow(deprecated)]
    fn description(&self) -> &str {
        "failed to join paths"
    }
}

pub fn current_exe() -> io::Result<PathBuf> {
    unsupported()
}

pub struct Env(Void);

impl Iterator for Env {
    type Item = (OsString, OsString);
    fn next(&mut self) -> Option<(OsString, OsString)> {
        match self.0 {}
    }
}

pub fn env() -> Env {
    panic!("not supported on this platform")
}

pub fn getenv(_: &OsStr) -> io::Result<Option<OsString>> {
    Ok(None)
}

pub fn setenv(_: &OsStr, _: &OsStr) -> io::Result<()> {
    Err(io::Error::new(io::ErrorKind::Other, "cannot set env vars on this platform"))
}

pub fn unsetenv(_: &OsStr) -> io::Result<()> {
    Err(io::Error::new(io::ErrorKind::Other, "cannot unset env vars on this platform"))
}

pub fn temp_dir() -> PathBuf {
    PathBuf::from("/tmp")
}

pub fn home_dir() -> Option<PathBuf> {
    None
}

pub fn exit(code: i32) -> ! {
    unsafe { libc::exit(code) }
}

pub fn getpid() -> u32 {
    0
}

pub fn page_size() -> usize {
    libc::PAGE_SIZE as usize
}

use crate::io;
use core::convert::TryInto;

pub struct Stdin;
pub struct Stdout;
pub struct Stderr;

impl Stdin {
    pub const fn new() -> Stdin {
        Stdin
    }
}

impl io::Read for Stdin {
    fn read(&mut self, data: &mut [u8]) -> io::Result<usize> {
        let bytes_read = unsafe {
            libc::readbytes(
                data.as_ptr() as *mut libc::c_char,
                (data.len() as libc::c_int).try_into().map_err(|_| {
                    io::Error::new(io::ErrorKind::InvalidInput, "Data length too long")
                })?,
            )
        };
        Ok(bytes_read as usize)
    }
}

impl Stdout {
    pub const fn new() -> Stdout {
        Stdout
    }
}

impl io::Write for Stdout {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        if data.len() > 0 {
            unsafe {
                for data in data.chunks(libc::c_int::MAX as usize) {
                    libc::printbytes(data.as_ptr() as *const libc::c_char, data.len() as libc::c_int)
                }
            }
        }
        Ok(data.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Stderr {
    pub const fn new() -> Stderr {
        Stderr
    }
}

impl io::Write for Stderr {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        if data.len() > 0 {
            unsafe {
                for data in data.chunks(libc::c_int::MAX as usize) {
                    libc::printbytes(data.as_ptr() as *const libc::c_char, data.len() as libc::c_int)
                }
            }
        }
        Ok(data.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub const STDIN_BUF_SIZE: usize = 0;

pub fn is_ebadf(_err: &io::Error) -> bool {
    true
}

pub fn panic_output() -> Option<impl io::Write> {
    Some(Stderr::new())
}

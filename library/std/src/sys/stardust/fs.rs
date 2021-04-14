use crate::env::current_dir;
use crate::ffi::{CStr, CString, OsString};
use crate::fmt;
use crate::hash::Hash;
use crate::io::{self, Error, ErrorKind, IoSlice, IoSliceMut, SeekFrom};
use crate::mem;
use crate::os::stardust::ffi::OsStringExt;
use crate::path::{Component, Path, PathBuf};
use crate::sync::{Arc, RwLock};
use crate::sys::time::SystemTime;
use crate::sys::unsupported;
use crate::sys_common::os_str_bytes::OsStrExt;
use libc::{DIR, FIL, FILINFO};

fn get_error(result: u32) -> io::Result<()> {
    Err(match result {
        0 => return Ok(()),
        1 => Error::new(ErrorKind::Other, "A hard error occurred in the low level disk I/O layer"),
        2 => Error::new(ErrorKind::Other, "Assertion failed"),
        3 => Error::new(ErrorKind::Other, "The physical drive cannot work"),
        4 => Error::new(ErrorKind::NotFound, "Could not find the file"),
        5 => Error::new(ErrorKind::NotFound, "Could not find the path"),
        6 => Error::new(ErrorKind::InvalidInput, "The path name format is invalid"),
        7 => Error::new(
            ErrorKind::PermissionDenied,
            "Access denied due to prohibited access or directory full",
        ),
        8 => Error::new(ErrorKind::PermissionDenied, "Access denied due to prohibited access"),
        9 => Error::new(ErrorKind::InvalidInput, "The file/directory object is invalid"),
        10 => Error::new(ErrorKind::Other, "The physical drive is write protected"),
        11 => Error::new(ErrorKind::Other, "The logical drive number is invalid"),
        12 => Error::new(ErrorKind::Other, "The volume has no work area"),
        13 => Error::new(ErrorKind::Other, "There is no valid FAT volume"),
        14 => Error::new(ErrorKind::Interrupted, "The f_mkfs() aborted due to any problem"),
        15 => Error::new(
            ErrorKind::TimedOut,
            "Could not get a grant to access the volume within defined period",
        ),
        16 => Error::new(
            ErrorKind::PermissionDenied,
            "The operation is rejected according to the file sharing policy",
        ),
        17 => Error::new(ErrorKind::Other, "LFN working buffer could not be allocated"),
        18 => Error::new(ErrorKind::Other, "Number of open files > FF_FS_LOCK"),
        19 => Error::new(ErrorKind::InvalidInput, "Given parameter is invalid"),
        _ => Error::new(ErrorKind::Other, "Unknown error"),
    })
}

pub struct File {
    file: Arc<RwLock<FileInner>>,
    path: CString,
}

struct FileInner(FIL);

unsafe impl Send for File {}
unsafe impl Sync for File {}

#[derive(Copy, Clone, Debug)]
pub struct FileAttr {
    size: u64,
    modified: SystemTime,
    permissions: FilePermissions,
    file_type: FileType,
}

pub struct ReadDir {
    dir: DIR,
    path: CString,
}

/*unsafe impl Send for ReadDir {}
unsafe impl Sync for ReadDir {}*/

pub struct DirEntry {
    filinfo: FILINFO,
    path: CString,
}

#[derive(Clone, Debug)]
pub struct OpenOptions {
    // generic
    read: bool,
    write: bool,
    append: bool,
    truncate: bool,
    create: bool,
    create_new: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct FilePermissions {
    read_only: bool,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct FileType(u8);

#[derive(Debug)]
pub struct DirBuilder {}

impl FileAttr {
    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn perm(&self) -> FilePermissions {
        self.permissions
    }

    pub fn file_type(&self) -> FileType {
        self.file_type
    }

    pub fn modified(&self) -> io::Result<SystemTime> {
        Ok(self.modified)
    }

    pub fn accessed(&self) -> io::Result<SystemTime> {
        Ok(self.modified)
    }

    pub fn created(&self) -> io::Result<SystemTime> {
        Ok(self.modified)
    }

    fn from_filinfo(info: &FILINFO) -> Self {
        FileAttr {
            size: info.fsize,
            modified: get_system_time(info.fdate, info.ftime),
            permissions: FilePermissions { read_only: (info.fattrib & libc::AM_RDO) > 0 },
            file_type: FileType(info.fattrib),
        }
    }
}

impl FilePermissions {
    pub fn readonly(&self) -> bool {
        self.read_only
    }

    pub fn set_readonly(&mut self, readonly: bool) {
        self.read_only = readonly
    }
}

impl FileType {
    pub fn is_dir(&self) -> bool {
        (self.0 & libc::AM_DIR) > 0
    }

    pub fn is_file(&self) -> bool {
        !self.is_dir()
    }

    pub fn is_symlink(&self) -> bool {
        false
    }
}

impl fmt::Debug for ReadDir {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.path.fmt(f)
    }
}

impl Iterator for ReadDir {
    type Item = io::Result<DirEntry>;

    fn next(&mut self) -> Option<io::Result<DirEntry>> {
        let mut filinfo: FILINFO = unsafe { mem::zeroed() };
        get_error(unsafe { libc::f_readdir(&mut self.dir, &mut filinfo) }).ok()?;

        if filinfo.fname[0] == 0 {
            None
        } else {
            Some(Ok(DirEntry { filinfo, path: self.path.clone() }))
        }
    }
}

impl DirEntry {
    pub fn path(&self) -> PathBuf {
        let root = OsString::from_vec(self.path.clone().into_bytes());
        let mut path = PathBuf::from(root);
        path.push(self.file_name());
        path
    }

    pub fn file_name(&self) -> OsString {
        let name = unsafe { CStr::from_ptr(&self.filinfo.fname as *const i8) }.to_owned();
        OsString::from_vec(name.into_bytes())
    }

    pub fn metadata(&self) -> io::Result<FileAttr> {
        Ok(FileAttr::from_filinfo(&self.filinfo))
    }

    pub fn file_type(&self) -> io::Result<FileType> {
        Ok(FileType(self.filinfo.fattrib))
    }
}

impl OpenOptions {
    pub fn new() -> OpenOptions {
        OpenOptions {
            read: false,
            write: false,
            append: false,
            truncate: false,
            create: false,
            create_new: false,
        }
    }

    pub fn read(&mut self, read: bool) {
        self.read = read;
    }
    pub fn write(&mut self, write: bool) {
        self.write = write;
    }
    pub fn append(&mut self, append: bool) {
        self.append = append;
    }
    pub fn truncate(&mut self, truncate: bool) {
        self.truncate = truncate;
    }
    pub fn create(&mut self, create: bool) {
        self.create = create;
    }
    pub fn create_new(&mut self, create_new: bool) {
        self.create_new = create_new;
    }

    fn get_mode(&self) -> io::Result<libc::BYTE> {
        match (self.write, self.append) {
            (true, false) => {}
            (false, false) => {
                if self.truncate || self.create || self.create_new {
                    return Err(Error::from_raw_os_error(libc::EINVAL));
                }
            }
            (_, true) => {
                if self.truncate && !self.create_new {
                    return Err(Error::from_raw_os_error(libc::EINVAL));
                }
            }
        }

        let creation_mode = match (self.create, self.append, self.create_new) {
            (false, false, false) => 0,
            (true, false, false) => libc::FA_CREATE_ALWAYS,
            (false, true, false) => libc::FA_SEEKEND,
            (true, true, false) => libc::FA_CREATE_ALWAYS | libc::FA_SEEKEND,
            (_, false, true) => libc::FA_CREATE_NEW,
            (_, true, true) => libc::FA_OPEN_APPEND,
        };

        let access_mode = match (self.read, self.write) {
            (false, false) => 0,
            (true, false) => libc::FA_READ,
            (false, true) => libc::FA_WRITE,
            (true, true) => libc::FA_READ | libc::FA_WRITE,
        };

        Ok(creation_mode | access_mode)
    }
}

fn cstr(path: &Path) -> io::Result<CString> {
    Ok(CString::new(path.as_os_str().as_bytes())?)
}

impl File {
    pub fn open(path: &Path, opts: &OpenOptions) -> io::Result<File> {
        let path = cstr(path)?;
        File::copen(path, opts)
    }

    fn copen(path: CString, opts: &OpenOptions) -> io::Result<File> {
        let mut fil: FIL = unsafe { mem::zeroed() };
        get_error(unsafe { libc::f_open(&mut fil as *mut FIL, path.as_ptr(), opts.get_mode()?) })?;
        Ok(File { file: Arc::new(RwLock::new(FileInner(fil))), path })
    }

    pub fn file_attr(&self) -> io::Result<FileAttr> {
        get_stat(&self.path)
    }

    pub fn fsync(&self) -> io::Result<()> {
        match self.file.write() {
            Ok(mut guard) => {
                let file: &mut FIL = &mut guard.0;
                get_error(unsafe { libc::f_sync(file as *mut FIL) })
            }
            Err(_) => Err(Error::new(ErrorKind::Other, "Lock poisoned"))
        }
    }

    pub fn datasync(&self) -> io::Result<()> {
        self.fsync()
    }

    pub fn truncate(&self, _size: u64) -> io::Result<()> {
        match self.file.write() {
            Ok(mut guard) => {
                let file: &mut FIL = &mut guard.0;
                get_error(unsafe { libc::f_truncate(file as *mut FIL) })
            }
            Err(_) => Err(Error::new(ErrorKind::Other, "Lock poisoned"))
        }
    }

    pub fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        let mut read: libc::UINT = 0;
        match self.file.write() {
            Ok(mut guard) => {
                let file: &mut FIL = &mut guard.0;
                get_error(unsafe { libc::f_read(file as *mut FIL, buf.as_mut_ptr() as *mut libc::c_void,
                                                buf.len() as libc::UINT,
                                                &mut read) })?;
                Ok(read as usize)
            }
            Err(_) => Err(Error::new(ErrorKind::Other, "Lock poisoned"))
        }
    }

    pub fn read_vectored(&self, _bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        Err(Error::new(ErrorKind::Other, "Not supported"))
    }

    pub fn is_read_vectored(&self) -> bool {
        false
    }

    pub fn write(&self, buf: &[u8]) -> io::Result<usize> {
        let mut written: libc::UINT = 0;

        match self.file.write() {
            Ok(mut guard) => {
                let file: &mut FIL = &mut guard.0;
                get_error(unsafe { libc::f_write(file as *mut FIL,
                                                 buf.as_ptr() as *const libc::c_void,
                                                 buf.len() as libc::UINT,
                                                 &mut written,) })?;
                Ok(written as usize)
            }
            Err(_) => Err(Error::new(ErrorKind::Other, "Lock poisoned"))
        }
    }

    pub fn write_vectored(&self, _bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        Err(Error::new(ErrorKind::Other, "Not supported"))
    }

    pub fn is_write_vectored(&self) -> bool {
        false
    }

    pub fn flush(&self) -> io::Result<()> {
        self.fsync()
    }

    pub fn seek(&self, pos: SeekFrom) -> io::Result<u64> {
        let position = match pos {
            SeekFrom::Start(p) => p,
            SeekFrom::End(p) => {
                let size = self.file_attr()?.size();
                if p > 0 {
                    size + p as u64
                } else {
                    size.checked_sub(p.abs() as u64).ok_or_else(|| {
                        Error::new(ErrorKind::Other, "Cannot seek beyond start of file")
                    })?
                }
            }
            SeekFrom::Current(p) => {
                let current = self.file.read().map_err(|_| Error::new(ErrorKind::Other, "Lock poisoned"))?.0.fptr;
                if p > 0 {
                    current + p as u64
                } else {
                    current.checked_sub(p.abs() as u64).ok_or_else(|| {
                        Error::new(ErrorKind::Other, "Cannot seek beyond start of file")
                    })?
                }
            }
        };

        match self.file.write() {
            Ok(mut guard) => {
                let file: &mut FIL = &mut guard.0;
                get_error(unsafe { libc::f_lseek(file as *mut FIL, position) })?;
            }
            Err(_) => return Err(Error::new(ErrorKind::Other, "Lock poisoned"))
        }
        let new_position = self.file.read().map_err(|_| Error::new(ErrorKind::Other, "Lock poisoned"))?.0.fptr;
        Ok(new_position)
    }

    pub fn duplicate(&self) -> io::Result<File> {
        Ok(Self { file: self.file.clone(), path: self.path.clone() })
    }

    pub fn set_permissions(&self, perm: FilePermissions) -> io::Result<()> {
        let attribute = if perm.read_only { libc::AM_RDO } else { 0 };
        get_error(unsafe { libc::f_chmod(self.path.as_ptr(), attribute, libc::AM_RDO) })
    }
}

impl Drop for FileInner {
    fn drop(&mut self) {
        let _ = unsafe { libc::f_close(&mut self.0 as *mut libc::FIL) };
    }
}

impl DirBuilder {
    pub fn new() -> DirBuilder {
        DirBuilder {}
    }

    pub fn mkdir(&self, p: &Path) -> io::Result<()> {
        let path = cstr(p)?;
        get_error(unsafe { libc::f_mkdir(path.as_ptr()) })
    }
}

impl fmt::Debug for File {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.path.fmt(f)
    }
}

pub fn readdir(p: &Path) -> io::Result<ReadDir> {
    let path = cstr(p)?;
    let mut dir: DIR = unsafe { mem::zeroed() };
    get_error(unsafe { libc::f_opendir(&mut dir, path.as_ptr()) })?;
    Ok(ReadDir { dir, path })
}

pub fn unlink(p: &Path) -> io::Result<()> {
    let path = cstr(p)?;
    let stat = get_stat(&path)?;
    if stat.file_type().is_file() {
        remove(&path)
    } else {
        Err(Error::new(ErrorKind::Other, "Not a file"))
    }
}

fn remove(path: &CStr) -> io::Result<()> {
    get_error(unsafe { libc::f_unlink(path.as_ptr()) })
}

fn get_stat(path: &CStr) -> io::Result<FileAttr> {
    let mut filinfo: FILINFO = unsafe { mem::zeroed() };
    get_error(unsafe { libc::f_stat(path.as_ptr(), &mut filinfo as *mut FILINFO) })?;
    Ok(FileAttr::from_filinfo(&filinfo))
}

pub fn rename(old: &Path, new: &Path) -> io::Result<()> {
    let old = cstr(old)?;
    let new = cstr(new)?;
    get_error(unsafe { libc::f_rename(old.as_ptr(), new.as_ptr()) })
}

pub fn set_perm(p: &Path, perm: FilePermissions) -> io::Result<()> {
    let path = cstr(p)?;
    let attribute = if perm.read_only { libc::AM_RDO } else { 0 };
    get_error(unsafe { libc::f_chmod(path.as_ptr(), attribute, libc::AM_RDO) })
}

pub fn rmdir(p: &Path) -> io::Result<()> {
    let path = cstr(p)?;
    let stat = get_stat(&path)?;
    if stat.file_type().is_dir() {
        remove(&path)
    } else {
        Err(Error::new(ErrorKind::Other, "Not a directory"))
    }
}

pub fn remove_dir_all(path: &Path) -> io::Result<()> {
    let dir = readdir(path)?;
    for entry in dir {
        let entry = entry?;
        let metadata = entry.metadata()?;
        if metadata.file_type().is_file() {
            let path = cstr(&entry.path())?;
            remove(&path)?
        } else {
            remove_dir_all(&entry.path())?
        }
    }
    let path = cstr(path)?;
    remove(&path)
}

pub fn readlink(_p: &Path) -> io::Result<PathBuf> {
    unsupported()
}

pub fn symlink(_original: &Path, _link: &Path) -> io::Result<()> {
    unsupported()
}

pub fn link(_src: &Path, _dst: &Path) -> io::Result<()> {
    unsupported()
}

pub fn stat(p: &Path) -> io::Result<FileAttr> {
    let path = cstr(p)?;
    get_stat(&path)
}

pub fn lstat(p: &Path) -> io::Result<FileAttr> {
    stat(p)
}

pub fn canonicalize(p: &Path) -> io::Result<PathBuf> {
    let mut buffer = PathBuf::with_capacity(p.as_os_str().len());
    for component in p.components() {
        match component {
            Component::Prefix(_) => {
                return Err(Error::new(ErrorKind::Other, "invalid path component"));
            }
            Component::RootDir => buffer.push(&component),
            Component::CurDir => {
                let current = current_dir()?;
                buffer.push(current);
            }
            Component::ParentDir => {
                if !buffer.pop() {
                    return Err(Error::new(ErrorKind::Other, "invalid path component"));
                }
            }
            Component::Normal(s) => buffer.push(s),
        }
    }
    Ok(buffer)
}

pub fn copy(from: &Path, to: &Path) -> io::Result<u64> {
    use crate::fs::File;
    use crate::fs::OpenOptions;
    let mut reader = File::open(from)?;
    let mut writer = OpenOptions::new().write(true).create(true).truncate(true).open(to)?;

    io::copy(&mut reader, &mut writer)
}

fn get_system_time(date: u16, time: u16) -> SystemTime {
    let mut year = ((date >> 9) + 1980) as u64;
    let mut month = ((date >> 5) & 15 + 1) as u64;
    let day = (date & 31) as u64;

    let hour = (time >> 11) as u64;
    let minute = ((time >> 5) & 63) as u64;
    let second = ((time & 31) >> 1) as u64;

    month = month.wrapping_sub(2);
    if (month as i32) < 0 {
        month += 12;
        year -= 1;
    }

    let elapsed_seconds =
        ((((year / 4 - year / 100 + year / 400 + 367 * month / 12 + day) + year * 365 - 719499)
            * 24
            + hour)
            * 60
            + minute)
            * 60
            + second;

    let ts = libc::timeval { tv_sec: elapsed_seconds as i64, tv_usec: 0 };
    ts.into()
}

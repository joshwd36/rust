use crate::cell::UnsafeCell;
use crate::sys::mutex::{self, Mutex};
use crate::time::Duration;

pub struct Condvar {
    inner: UnsafeCell<libc::pthread_cond_t>,
}

pub type MovableCondvar = Box<Condvar>;

unsafe impl Send for Condvar {}
unsafe impl Sync for Condvar {}

const TIMESPEC_MAX: libc::timespec =
    libc::timespec { tv_sec: <libc::time_t>::MAX, tv_nsec: 1_000_000_000 - 1 };

fn saturating_cast_to_time_t(value: u64) -> libc::time_t {
    if value > <libc::time_t>::MAX as u64 { <libc::time_t>::MAX } else { value as libc::time_t }
}

impl Condvar {
    pub const fn new() -> Condvar {
        // Might be moved and address is changing it is better to avoid
        // initialization of potentially opaque OS data before it landed
        Condvar { inner: UnsafeCell::new(libc::PTHREAD_COND_INITIALIZER as libc::pthread_cond_t) }
    }

    pub unsafe fn init(&mut self) {}

    #[inline]
    pub unsafe fn notify_one(&self) {
        let r = libc::pthread_cond_signal(self.inner.get());
        debug_assert_eq!(r, 0);
    }

    #[inline]
    pub unsafe fn notify_all(&self) {
        let r = libc::pthread_cond_broadcast(self.inner.get());
        debug_assert_eq!(r, 0);
    }

    #[inline]
    pub unsafe fn wait(&self, mutex: &Mutex) {
        let r = libc::pthread_cond_wait(self.inner.get(), mutex::raw(mutex));
        debug_assert_eq!(r, 0);
    }

    // This implementation is modeled after libcxx's condition_variable
    // https://github.com/llvm-mirror/libcxx/blob/release_35/src/condition_variable.cpp#L46
    // https://github.com/llvm-mirror/libcxx/blob/release_35/include/__mutex_base#L367
    pub unsafe fn wait_timeout(&self, mutex: &Mutex, mut dur: Duration) -> bool {
        use crate::time::Instant;

        // 1000 years
        let max_dur = Duration::from_secs(1000 * 365 * 86400);

        if dur > max_dur {
            // OSX implementation of `pthread_cond_timedwait` is buggy
            // with super long durations. When duration is greater than
            // 0x100_0000_0000_0000 seconds, `pthread_cond_timedwait`
            // in macOS Sierra return error 316.
            //
            // This program demonstrates the issue:
            // https://gist.github.com/stepancheg/198db4623a20aad2ad7cddb8fda4a63c
            //
            // To work around this issue, and possible bugs of other OSes, timeout
            // is clamped to 1000 years, which is allowable per the API of `wait_timeout`
            // because of spurious wakeups.

            dur = max_dur;
        }

        // First, figure out what time it currently is, in both system and
        // stable time.  pthread_cond_timedwait uses system time, but we want to
        // report timeout based on stable time.
        let mut sys_now = libc::timeval { tv_sec: 0, tv_usec: 0 };
        let stable_now = Instant::now();
        let r = libc::gettimeofday(&mut sys_now);
        debug_assert_eq!(r, 0);

        let nsec = dur.subsec_nanos() as libc::c_long + (sys_now.tv_usec * 1000) as libc::c_long;
        let extra = (nsec / 1_000_000_000) as libc::time_t;
        let nsec = nsec % 1_000_000_000;
        let seconds = saturating_cast_to_time_t(dur.as_secs());

        let timeout = sys_now
            .tv_sec
            .checked_add(extra)
            .and_then(|s| s.checked_add(seconds))
            .map(|s| libc::timespec { tv_sec: s, tv_nsec: nsec })
            .unwrap_or(TIMESPEC_MAX);

        // And wait!
        let r = libc::pthread_cond_timedwait(self.inner.get(), mutex::raw(mutex), &timeout);
        debug_assert!(r == libc::ETIMEDOUT || r == 0);

        // ETIMEDOUT is not a totally reliable method of determining timeout due
        // to clock shifts, so do the check ourselves
        stable_now.elapsed() < dur
    }

    #[inline]
    pub unsafe fn destroy(&self) {
        let r = libc::pthread_cond_destroy(self.inner.get());
        debug_assert_eq!(r, 0);
    }
}

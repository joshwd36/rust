#![allow(dead_code)]
use crate::time::Duration;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct Instant(Duration);

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct SystemTime(Duration);

pub const UNIX_EPOCH: SystemTime = SystemTime(Duration::from_secs(0));

fn current_time(_clock_id: u32) -> Duration {
    let mut tv = libc::timeval { tv_sec: 0, tv_usec: 0 };
    let result = unsafe { libc::gettimeofday(&mut tv) };
    if result == 0 {
        Duration::new(tv.tv_sec as u64, (tv.tv_usec * 1000) as u32)
    } else {
        Duration::from_secs(0)
    }
}

impl Instant {
    pub fn now() -> Instant {
        Instant(current_time(libc::CLOCK_MONOTONIC))
    }

    pub const fn zero() -> Instant {
        Instant(Duration::from_secs(0))
    }

    pub fn actually_monotonic() -> bool {
        true
    }

    pub fn checked_sub_instant(&self, other: &Instant) -> Option<Duration> {
        self.0.checked_sub(other.0)
    }

    pub fn checked_add_duration(&self, other: &Duration) -> Option<Instant> {
        Some(Instant(self.0.checked_add(*other)?))
    }

    pub fn checked_sub_duration(&self, other: &Duration) -> Option<Instant> {
        Some(Instant(self.0.checked_sub(*other)?))
    }
}

impl SystemTime {
    pub fn now() -> SystemTime {
        SystemTime(current_time(libc::CLOCK_REALTIME))
    }

    pub fn sub_time(&self, other: &SystemTime) -> Result<Duration, Duration> {
        self.0.checked_sub(other.0).ok_or_else(|| other.0 - self.0)
    }

    pub fn checked_add_duration(&self, other: &Duration) -> Option<SystemTime> {
        Some(SystemTime(self.0.checked_add(*other)?))
    }

    pub fn checked_sub_duration(&self, other: &Duration) -> Option<SystemTime> {
        Some(SystemTime(self.0.checked_sub(*other)?))
    }
}

impl From<libc::timeval> for SystemTime {
    fn from(tv: libc::timeval) -> SystemTime {
        SystemTime { 0: Duration::new(tv.tv_sec as u64, (tv.tv_usec * 1000) as u32) }
    }
}

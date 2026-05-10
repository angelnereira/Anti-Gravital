use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

/// Lightweight notification primitive that signals Go workers when new READY
/// slots appear in the ring buffer.
///
/// On Linux the implementation uses a futex, which costs zero kernel transitions
/// when there are no waiters (the common case under load). On non-Linux Unix
/// systems it falls back to a `std::sync::Condvar`.
pub struct Notifier {
    #[cfg(target_os = "linux")]
    word: AtomicU32,
    #[cfg(not(target_os = "linux"))]
    inner: std::sync::Mutex<u32>,
    #[cfg(not(target_os = "linux"))]
    cvar: std::sync::Condvar,
}

impl Notifier {
    pub fn new() -> Self {
        #[cfg(target_os = "linux")]
        {
            Self { word: AtomicU32::new(0) }
        }
        #[cfg(not(target_os = "linux"))]
        {
            Self {
                inner: std::sync::Mutex::new(0),
                cvar: std::sync::Condvar::new(),
            }
        }
    }

    /// Returns the current sequence counter for use as `expected` in [`wait`].
    pub fn current(&self) -> u32 {
        #[cfg(target_os = "linux")]
        {
            self.word.load(Ordering::Acquire)
        }
        #[cfg(not(target_os = "linux"))]
        {
            *self.inner.lock().unwrap()
        }
    }

    /// Wakes one waiter.
    pub fn notify_one(&self) {
        #[cfg(target_os = "linux")]
        {
            self.word.fetch_add(1, Ordering::Release);
            futex_wake(&self.word, 1);
        }
        #[cfg(not(target_os = "linux"))]
        {
            *self.inner.lock().unwrap() += 1;
            self.cvar.notify_one();
        }
    }

    /// Wakes all waiters.
    pub fn notify_all(&self) {
        #[cfg(target_os = "linux")]
        {
            self.word.fetch_add(1, Ordering::Release);
            futex_wake(&self.word, i32::MAX);
        }
        #[cfg(not(target_os = "linux"))]
        {
            *self.inner.lock().unwrap() += 1;
            self.cvar.notify_all();
        }
    }

    /// Blocks until notified or `timeout` elapses.
    ///
    /// Returns `true` if woken by a notification, `false` on timeout.
    pub fn wait(&self, expected: u32, timeout: Option<Duration>) -> bool {
        #[cfg(target_os = "linux")]
        {
            futex_wait(&self.word, expected, timeout)
        }
        #[cfg(not(target_os = "linux"))]
        {
            let guard = self.inner.lock().unwrap();
            let result = if let Some(dur) = timeout {
                self.cvar.wait_timeout_while(guard, dur, |v| *v == expected).unwrap()
            } else {
                let g = self.cvar.wait_while(guard, |v| *v == expected).unwrap();
                return *g != expected;
            };
            !result.1.timed_out()
        }
    }
}

impl Default for Notifier {
    fn default() -> Self {
        Self::new()
    }
}

// ── Linux futex helpers ──────────────────────────────────────────────────────

#[cfg(target_os = "linux")]
fn futex_wake(word: &AtomicU32, count: i32) {
    // SAFETY: FUTEX_WAKE on a valid aligned AtomicU32 is always safe.
    unsafe {
        libc::syscall(
            libc::SYS_futex,
            word as *const AtomicU32,
            libc::FUTEX_WAKE | libc::FUTEX_PRIVATE_FLAG,
            count,
            std::ptr::null::<libc::timespec>(),
            std::ptr::null::<u32>(),
            0u32,
        );
    }
}

#[cfg(target_os = "linux")]
fn futex_wait(word: &AtomicU32, expected: u32, timeout: Option<Duration>) -> bool {
    let ts = timeout.map(|d| libc::timespec {
        tv_sec: d.as_secs() as libc::time_t,
        tv_nsec: d.subsec_nanos() as libc::c_long,
    });
    let ts_ptr = ts.as_ref().map_or(std::ptr::null(), |t| t as *const libc::timespec);

    // SAFETY: FUTEX_WAIT on a valid aligned AtomicU32 is safe. The kernel
    // atomically checks that *word == expected before sleeping.
    let rc = unsafe {
        libc::syscall(
            libc::SYS_futex,
            word as *const AtomicU32,
            libc::FUTEX_WAIT | libc::FUTEX_PRIVATE_FLAG,
            expected,
            ts_ptr,
            std::ptr::null::<u32>(),
            0u32,
        )
    } as i32;

    // rc == 0: woken by FUTEX_WAKE
    // rc == -EAGAIN: value already changed before we slept (counts as woken)
    // rc == -ETIMEDOUT: deadline expired
    rc == 0 || rc == -libc::EAGAIN
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn notify_wakes_waiter() {
        let n = Arc::new(Notifier::new());
        let n2 = Arc::clone(&n);
        let expected = n.current();

        let handle = thread::spawn(move || n2.wait(expected, Some(Duration::from_secs(2))));

        thread::sleep(Duration::from_millis(20));
        n.notify_one();

        assert!(handle.join().unwrap(), "waiter should have been woken");
    }

    #[test]
    fn wait_times_out() {
        let n = Notifier::new();
        let expected = n.current();
        let woken = n.wait(expected, Some(Duration::from_millis(50)));
        assert!(!woken, "no notification was sent; wait should have timed out");
    }
}

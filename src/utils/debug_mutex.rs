use std::sync::{LockResult, Mutex, MutexGuard};

pub struct DebugMutex<T> {
    name: &'static str,
    mtx: Mutex<T>,
}

impl<T> DebugMutex<T> {
    pub fn new(inner: T, name: &'static str) -> Self {
        DebugMutex {
            mtx: Mutex::new(inner),
            name,
        }
    }
    pub fn lock(&self, reason: &'static str) -> LockResult<MutexGuard<T>> {
        log::trace!(
            "lock mutex:{} for: {} from thread: {}",
            self.name,
            reason,
            thread_id::get()
        );
        let ret = self.mtx.lock();
        log::trace!(
            "locked mutex:{} for: {} from thread: {}",
            self.name,
            reason,
            thread_id::get()
        );
        ret

        // first try_lock

        // if locked by other thread

        // trace!("lock mutex:{} for: {} from thread: {} -IS LOCKED- wait for thread: {}, locked_reason: {}", self.name, reason);

        // if locked by same thread

        // panic!("LOCKED BY SELF: lock mutex:{} for: {} from thread: {} -IS LOCKED- wait for thread: {}, locked_reason: {}", self.name, reason);
    }
}

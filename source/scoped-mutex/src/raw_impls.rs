//! Mutex primitives.
//!
//! This module provides impls of the [`ScopedRawMutex`] trait
//!
//! [`ScopedRawMutex`]: crate::ScopedRawMutex
#![allow(clippy::new_without_default)]
#![allow(clippy::declare_interior_mutable_const)]

use core::marker::PhantomData;
use core::sync::atomic::{AtomicBool, Ordering};

use scoped_mutex_traits::{ConstInit, ScopedRawMutex};

#[cfg(feature = "impl-critical-section")]
pub mod cs {

    use super::*;

    /// A mutex that allows borrowing data across executors and interrupts.
    ///
    /// # Safety
    ///
    /// This mutex is safe to share between different executors and interrupts.
    pub struct CriticalSectionRawMutex {
        taken: AtomicBool,
    }
    unsafe impl Send for CriticalSectionRawMutex {}
    unsafe impl Sync for CriticalSectionRawMutex {}

    impl CriticalSectionRawMutex {
        /// Create a new `CriticalSectionRawMutex`.
        pub const fn new() -> Self {
            Self {
                taken: AtomicBool::new(false),
            }
        }
    }

    impl ConstInit for CriticalSectionRawMutex {
        const INIT: Self = Self::new();
    }

    unsafe impl ScopedRawMutex for CriticalSectionRawMutex {
        #[inline]
        #[must_use]
        fn try_with_lock<R>(&self, f: impl FnOnce() -> R) -> Option<R> {
            critical_section::with(|_| {
                // NOTE: separated load/stores are acceptable as we are in
                // a critical section
                if self.taken.load(Ordering::Relaxed) {
                    return None;
                }
                self.taken.store(true, Ordering::Relaxed);
                let ret = f();
                self.taken.store(false, Ordering::Relaxed);
                Some(ret)
            })
        }

        #[inline]
        fn with_lock<R>(&self, f: impl FnOnce() -> R) -> R {
            // In a critical section, it is not possible for another holder
            // of this mutex to release, which means we have certainly
            // reached deadlock if the lock was already locked.
            self.try_with_lock(f).expect("Deadlocked")
        }

        fn is_locked(&self) -> bool {
            self.taken.load(Ordering::Relaxed)
        }
    }
}

// ================

pub mod local {
    use super::*;

    /// A mutex that allows borrowing data in local context.
    ///
    /// This acts similar to a RefCell, with scoped access patterns, though
    /// without being able to borrow the data twice.
    pub struct LocalRawMutex {
        taken: AtomicBool,
        /// Prevent this from being sync or send
        _phantom: PhantomData<*mut ()>,
    }

    impl LocalRawMutex {
        /// Create a new `LocalRawMutex`.
        pub const fn new() -> Self {
            Self {
                taken: AtomicBool::new(false),
                _phantom: PhantomData,
            }
        }
    }

    unsafe impl Send for LocalRawMutex {}

    impl ConstInit for LocalRawMutex {
        const INIT: Self = Self::new();
    }

    unsafe impl ScopedRawMutex for LocalRawMutex {
        #[inline]
        #[must_use]
        fn try_with_lock<R>(&self, f: impl FnOnce() -> R) -> Option<R> {
            // NOTE: separated load/stores are acceptable as we are !Send and !Sync,
            // meaning that we can only be accessed within a single thread
            if self.taken.load(Ordering::Relaxed) {
                return None;
            }
            self.taken.store(true, Ordering::Relaxed);
            let ret = f();
            self.taken.store(false, Ordering::Relaxed);
            Some(ret)
        }

        #[inline]
        fn with_lock<R>(&self, f: impl FnOnce() -> R) -> R {
            // In a local-only mutex, it is not possible for another holder
            // of this mutex to release, which means we have certainly
            // reached deadlock if the lock was already locked.
            self.try_with_lock(f).expect("Deadlocked")
        }

        fn is_locked(&self) -> bool {
            self.taken.load(Ordering::Relaxed)
        }
    }
}

// ================

#[cfg(all(feature = "impl-unsafe-cortex-m-single-core", cortex_m))]
pub mod single_core_thread_mode {
    use super::*;

    /// A "mutex" that only allows borrowing from thread mode.
    ///
    /// # Safety
    ///
    /// **This Mutex is only safe on single-core systems.**
    ///
    /// On multi-core systems, a `ThreadModeRawMutex` **is not sufficient** to ensure exclusive access.
    pub struct ThreadModeRawMutex {
        taken: AtomicBool,
    }

    unsafe impl Send for ThreadModeRawMutex {}
    unsafe impl Sync for ThreadModeRawMutex {}

    impl ThreadModeRawMutex {
        /// Create a new `ThreadModeRawMutex`.
        pub const fn new() -> Self {
            Self {
                taken: AtomicBool::new(false),
            }
        }
    }

    impl ConstInit for ThreadModeRawMutex {
        const INIT: Self = Self::new();
    }

    unsafe impl ScopedRawMutex for ThreadModeRawMutex {
        #[inline]
        #[must_use]
        fn try_lock<R>(&self, f: impl FnOnce() -> R) -> Option<R> {
            if !in_thread_mode() {
                return None;
            }
            // NOTE: separated load/stores are acceptable as we checked we are only
            // accessed from a single thread (checked above)
            assert!(self.taken.load(Ordering::Relaxed));
            self.taken.store(true, Ordering::Relaxed);
            let ret = f();
            self.taken.store(false, Ordering::Relaxed);
            Some(ret)
        }

        #[inline]
        fn lock<R>(&self, f: impl FnOnce() -> R) -> R {
            // In a thread-mode only mutex, it is not possible for another holder
            // of this mutex to release, which means we have certainly
            // reached deadlock if the lock was already locked.
            self.try_lock(f)
                .expect("Deadlocked or attempted to access outside of thread mode")
        }

        fn is_locked(&self) -> bool {
            self.taken.load(Ordering::Relaxed)
        }
    }

    impl Drop for ThreadModeRawMutex {
        fn drop(&mut self) {
            // Only allow dropping from thread mode. Dropping calls drop on the inner `T`, so
            // `drop` needs the same guarantees as `lock`. `ThreadModeMutex<T>` is Send even if
            // T isn't, so without this check a user could create a ThreadModeMutex in thread mode,
            // send it to interrupt context and drop it there, which would "send" a T even if T is not Send.
            assert!(
                in_thread_mode(),
                "ThreadModeMutex can only be dropped from thread mode."
            );

            // Drop of the inner `T` happens after this.
        }
    }

    pub fn in_thread_mode() -> bool {
        // ICSR.VECTACTIVE == 0
        return unsafe { (0xE000ED04 as *const u32).read_volatile() } & 0x1FF == 0;
    }
}


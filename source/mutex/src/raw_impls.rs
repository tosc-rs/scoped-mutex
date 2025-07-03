//! Mutex primitives.
//!
//! This module provides impls of the [`ScopedRawMutex`] and [`RawMutex`]
//! traits.
//!
//! [`ScopedRawMutex`]: crate::ScopedRawMutex
//! [`RawMutex`]: crate::RawMutex
//!
//! ## Which impl should I use?
//!
//! This module contains different implementations based on their capabilities and what
//! features or dependencies they require.
//!
//! ### `local` impl
//!
//! The [`local`] module implements a mutex that acts similarly to a `RefCell`, and is
//! useful when a type requires a [`ScopedRawMutex`], but the item is only ever used locally
//! within the same thread or task. This implementation does NOT impl `Sync`, so it cannot
//! be placed in a static. Because this property is checkable at compile time, this mutex
//! has the lowest overall cost to lock/unlock.
//!
//! This module requires no features to be enabled.
//!
//! ### `single_core_thread_mode` impl
//!
//! The [`single_core_thread_mode`] module implements a mutex that DOES impl `Sync`, but
//! only allows access when NOT in interrupt mode, e.g. "thread mode" on cortex-m devices.
//! This property is useful when you'd like to place a mutex in a static for lifetime or
//! accessibility reasons, but don't want to require a critical section to access, if all
//! access is done outside of interrupt context. This requires a runtime check, making it
//! a bit less efficient than the [`local`] impl, but less costly or disruptive than
//! taking a critical section.
//!
//! This impl is currently only usable on bare metal cortex-m targets, as it checks the
//! `ICSR.VECTACTIVE` field at runtime. PRs to allow similar checks for other architectures
//! are welcome. This impl ALSO requires the `impl-unsafe-cortex-m-single-core` feature to
//! be active, which should ONLY be enabled if your target is single core or is only being
//! used in a single core configuration, otherwise both cores could unsoundly gain access
//! at the same time.
//!
//! ### `cs` impl
//!
//! The [`cs`] module implements a mutex that DOES impl `Sync`, and provides exclusive
//! access using a critical section via the `critical-section` crate. The `critical-section`
//! crate allows users to define how a critical section can be obtained: on single core embedded
//! platforms, this usually involves disabling interrupts for the duration of the critical
//! section. On multicore embedded platforms, this usually involves using some hardware
//! synchronization utility, for example using the "Spinlock" peripheral on RP2xxx targets.
//! On desktop/`std` platforms, this typically involves using a `std` Mutex from the operating
//! system, which will prevent concurrent access.
//!
//! This impl can be used in the widest variety of cases, but generally has the highest cost
//! or largest impact to scheduling/latency. That being said: simply taking or releasing critical
//! sections is rarely an expensive operation (only a few cycles on embedded targets), however if an
//! expensive operation is done WHILE holding the critical section, latency of servicing interrupts
//! or other threads may be impacted.
//!
//! This impl requires the `impl-critical-section` feature to be active, and requires that a
//! `critical-section` implementation has been provided. If both the `std` and `impl-critical-section`
//! features of this crate are active, the `critical-section/std` feature is enabled, fulfilling this
//! requirement on std targets. For embedded targets, `critical-section` impls are usually provided
//! by your architecture crate (e.g. `cortex-m`) for single core targets or HAL crate
//! (e.g. `embassy-rp`) for multi-core targets.
//!
//! ### `lock_api_0_4`
//!
//! The `lock_api_0_4` module implements a mutex based on the `lock_api` crate. This is provided
//! for compatibility if your system is using mutexes based on the `lock_api`/`parking_lot`
//! crates. This is uncommon for embedded targets.
#![allow(clippy::new_without_default)]
#![allow(clippy::declare_interior_mutable_const)]

use core::marker::PhantomData;
use core::sync::atomic::{AtomicBool, Ordering};
use mutex_traits::{ConstInit, ScopedRawMutex};

pub use mutex_traits as traits;

#[cfg(feature = "impl-critical-section")]
pub mod cs {
    //! Critical Section based implementation

    use super::*;

    /// A mutex that allows borrowing data across executors and interrupts.
    ///
    /// # Safety
    ///
    /// This mutex is safe to share between different executors and interrupts.
    #[cfg_attr(feature = "fmt", derive(Debug))]
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
    //! Locally usable based implementation
    use super::*;

    /// A mutex that allows borrowing data in local context.
    ///
    /// This acts similar to a RefCell, with scoped access patterns, though
    /// without being able to borrow the data twice.
    #[cfg_attr(feature = "fmt", derive(Debug))]
    pub struct LocalRawMutex {
        taken: AtomicBool,
        /// Prevent this from being sync
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
            // NOTE: separated load/stores are acceptable as we are !Sync,
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
    //! A single-core safe implementation that does not require a critical section

    use super::*;

    /// A "mutex" that only allows borrowing from thread mode.
    ///
    /// # Safety
    ///
    /// **This Mutex is only safe on single-core systems.**
    ///
    /// On multi-core systems, a `ThreadModeRawMutex` **is not sufficient** to ensure exclusive access.
    #[cfg_attr(feature = "fmt", derive(Debug))]
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
        fn try_with_lock<R>(&self, f: impl FnOnce() -> R) -> Option<R> {
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
        fn with_lock<R>(&self, f: impl FnOnce() -> R) -> R {
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

    fn in_thread_mode() -> bool {
        // ICSR.VECTACTIVE == 0
        return unsafe { (0xE000ED04 as *const u32).read_volatile() } & 0x1FF == 0;
    }
}

#[cfg(feature = "impl-lock_api-0_4")]
pub mod lock_api_0_4 {
    //! [`lock_api`](https://crates.io/crates/lock_api) v0.4 [`RawMutex`]
    //! implementation.

    use ::lock_api_0_4 as lock_api;
    use mutex_traits::{ConstInit, RawMutex};

    /// [`lock_api`](https://crates.io/crates/lock_api) v0.4 [`RawMutex`]
    /// implementation.
    #[cfg_attr(feature = "fmt", derive(Debug))]
    pub struct LockApiRawMutex<T>(T);

    impl<T: lock_api::RawMutex> ConstInit for LockApiRawMutex<T> {
        const INIT: Self = LockApiRawMutex(T::INIT);
    }

    unsafe impl<T: lock_api::RawMutex> RawMutex for LockApiRawMutex<T> {
        type GuardMarker = <T as lock_api::RawMutex>::GuardMarker;

        #[inline]
        #[track_caller]
        fn lock(&self) {
            self.0.lock();
        }

        #[inline]
        #[track_caller]
        fn try_lock(&self) -> bool {
            self.0.try_lock()
        }

        #[inline]
        #[track_caller]
        unsafe fn unlock(&self) {
            self.0.unlock()
        }

        #[inline]
        #[track_caller]
        fn is_locked(&self) -> bool {
            self.0.is_locked()
        }
    }
}

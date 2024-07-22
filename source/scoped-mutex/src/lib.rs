//! Scoped Mutex Crate
#![deny(missing_docs)]
#![cfg_attr(not(feature = "std"), no_std)]

pub mod raw_impls;

pub use mutex_traits::{ConstInit, ScopedRawMutex};
use core::cell::UnsafeCell;

/// Blocking mutex (not async)
///
/// Provides a blocking mutual exclusion primitive backed by an implementation of [`ScopedRawMutex`].
///
/// Which implementation you select depends on the context in which you're using the mutex, and you can choose which kind
/// of interior mutability fits your use case.
///
/// Use [`CriticalSectionRawMutex`] when data can be shared between threads and interrupts.
///
/// Use [`LocalRawMutex`] when data is only shared between tasks running on the same executor.
///
/// Use [`ThreadModeRawMutex`] when data is shared between tasks running on the same executor but you want a global singleton.
///
/// In all cases, the blocking mutex is intended to be short lived and not held across await points.
///
/// [`CriticalSectionRawMutex`]: crate::raw_impls::cs::CriticalSectionRawMutex
/// [`LocalRawMutex`]: crate::raw_impls::local::LocalRawMutex
/// [`ThreadModeRawMutex`]: crate::raw_impls::single_core_thread_mode::ThreadModeRawMutex
pub struct BlockingMutex<R, T: ?Sized> {
    // NOTE: `raw` must be FIRST, so when using ThreadModeMutex the "can't drop in non-thread-mode" gets
    // to run BEFORE dropping `data`.
    raw: R,
    data: UnsafeCell<T>,
}

unsafe impl<R: ScopedRawMutex + Send, T: ?Sized + Send> Send for BlockingMutex<R, T> {}
unsafe impl<R: ScopedRawMutex + Sync, T: ?Sized + Send> Sync for BlockingMutex<R, T> {}

impl<R: ConstInit, T> BlockingMutex<R, T> {
    /// Creates a new mutex in an unlocked state ready for use.
    #[inline]
    pub const fn new(val: T) -> BlockingMutex<R, T> {
        BlockingMutex {
            raw: R::INIT,
            data: UnsafeCell::new(val),
        }
    }
}

impl<R: ScopedRawMutex, T> BlockingMutex<R, T> {
    /// Locks the raw mutex and grants temporary access to the inner data
    ///
    /// Behavior when the lock is already locked is dependent on the behavior
    /// of the Raw mutex. See [`ScopedRawMutex::with_lock()`]'s documentation for
    /// more details
    pub fn with_lock<U>(&self, f: impl FnOnce(&mut T) -> U) -> U {
        self.raw.with_lock(|| {
            let ptr = self.data.get();
            // SAFETY: Raw Mutex proves we have exclusive access to the inner data
            let inner = unsafe { &mut *ptr };
            f(inner)
        })
    }

    /// Locks the raw mutex and grants temporary access to the inner data
    ///
    /// Returns `Some(U)` if the lock was obtained. Returns `None` if the lock
    /// was already locked
    #[must_use]
    pub fn try_with_lock<U>(&self, f: impl FnOnce(&mut T) -> U) -> Option<U> {
        self.raw.try_with_lock(|| {
            let ptr = self.data.get();
            // SAFETY: Raw Mutex proves we have exclusive access to the inner data
            let inner = unsafe { &mut *ptr };
            f(inner)
        })
    }
}

impl<R, T> BlockingMutex<R, T> {
    /// Creates a new mutex based on a pre-existing raw mutex.
    ///
    /// This allows creating a mutex in a constant context on stable Rust.
    #[inline]
    pub const fn const_new(raw_mutex: R, val: T) -> BlockingMutex<R, T> {
        BlockingMutex {
            raw: raw_mutex,
            data: UnsafeCell::new(val),
        }
    }

    /// Consumes this mutex, returning the underlying data.
    #[inline]
    pub fn into_inner(self) -> T {
        self.data.into_inner()
    }

    /// Returns a mutable reference to the underlying data.
    ///
    /// Since this call borrows the `Mutex` mutably, no actual locking needs to
    /// take place---the mutable borrow statically guarantees no locks exist.
    #[inline]
    pub fn get_mut(&mut self) -> &mut T {
        unsafe { &mut *self.data.get() }
    }

    /// Returns a pointer to the inner storage
    ///
    /// # Safety
    ///
    /// Must NOT be called when the lock is taken
    pub unsafe fn get_unchecked(&self) -> *mut T {
        self.data.get()
    }
}

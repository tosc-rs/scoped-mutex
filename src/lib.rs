#![cfg_attr(not(feature = "std"), no_std)]

use core::cell::UnsafeCell;

pub mod impls;

pub trait ConstInit {
    /// Create a new instance.
    ///
    /// This is a const instead of a method to allow creating instances in const context.
    const INIT: Self;
}

/// Raw mutex trait.
///
/// This mutex is "raw", which means it does not actually contain the protected data, it
/// just implements the mutex mechanism. For most uses you should use [`BlockingMutex`]
/// instead, which is generic over a ScopedRawMutex and contains the protected data.
///
/// # Safety
///
/// ScopedRawMutex implementations must ensure that, while locked, no other thread can lock
/// the RawMutex concurrently. This can usually be implemented using an [`AtomicBool`]
/// to track the "taken" state. See [crate::impls] for examples of correct implementations.
///
/// Unsafe code is allowed to rely on this fact, so incorrect implementations will cause undefined behavior.
///
/// [`AtomicBool`]: core::sync::atomic::AtomicBool
pub unsafe trait ScopedRawMutex {
    /// Lock this `ScopedRawMutex`, calling `f()` after the lock has been acquired, and releasing
    /// the lock after the completion of `f()`.
    ///
    /// If this was successful, `Some(R)` will be returned. If the mutex was already locked,
    /// `None` will be returned
    #[must_use]
    fn try_lock<R>(&self, f: impl FnOnce() -> R) -> Option<R>;

    /// Lock this `ScopedRawMutex`, calling `f()` after the lock has been acquired, and releasing
    /// the lock after the completion of `f()`.
    ///
    /// Implementors may choose whether to block or panic if the lock is already locked.
    /// It is recommended to panic if it is possible to know that deadlock has occurred.
    ///
    /// For implementations on a system with threads, blocking may be the correct choice.
    ///
    /// For implementations where a single thread is present, panicking immediately may be
    /// the correct choice.
    fn lock<R>(&self, f: impl FnOnce() -> R) -> R;
}

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
/// [`CriticalSectionRawMutex`]: crate::impls::cs::CriticalSectionRawMutex
/// [`LocalRawMutex`]: crate::impls::local::LocalRawMutex
/// [`ThreadModeRawMutex`]: crate::impls::thread_mode::ThreadModeRawMutex
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
    /// of the Raw mutex. See [`ScopedRawMutex::lock()`]'s documentation for
    /// more details
    pub fn lock<U>(&self, f: impl FnOnce(&mut T) -> U) -> U {
        self.raw.lock(|| {
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
    pub fn try_lock<U>(&self, f: impl FnOnce(&mut T) -> U) -> Option<U> {
        self.raw.try_lock(|| {
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

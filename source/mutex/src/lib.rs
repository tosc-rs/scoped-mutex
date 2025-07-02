#![doc = include_str!("../README.md")]
#![deny(missing_docs)]
#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![cfg_attr(feature = "fmt", warn(missing_debug_implementations))]

pub mod raw_impls;

use core::cell::UnsafeCell;
use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};
use core::panic::AssertUnwindSafe;
pub use mutex_traits::{ConstInit, RawMutex, ScopedRawMutex};

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
/// Use [`LockApiRawMutex`] if you wish to use a type implementing
/// [`lock_api::RawMutex`] as the mutex implementation.
///
/// In all cases, the blocking mutex is intended to be short lived and not held across await points.
///
/// [`CriticalSectionRawMutex`]: crate::raw_impls::cs::CriticalSectionRawMutex
/// [`LocalRawMutex`]: crate::raw_impls::local::LocalRawMutex
/// [`ThreadModeRawMutex`]:
///     crate::raw_impls::single_core_thread_mode::ThreadModeRawMutex
/// [`LockApiRawMutex`]: crate::raw_impls::lock_api_0_4::LockApiRawMutex
/// [`lock_api::RawMutex`]:
///     https://docs.rs/lock_api/0.4.0/lock_api/trait.RawMutex.html
pub struct BlockingMutex<R, T: ?Sized> {
    // NOTE: `raw` must be FIRST, so when using ThreadModeMutex the "can't drop in non-thread-mode" gets
    // to run BEFORE dropping `data`.
    raw: R,
    data: UnsafeCell<T>,
}

/// A RAII guard that allows access to the data guarded by a [`BlockingMutex`].
#[must_use]
pub struct MutexGuard<'mutex, R: RawMutex, T: ?Sized> {
    lock: &'mutex BlockingMutex<R, T>,
    /// This marker makes the guard `Send` or `!Send` based on the `RawMutex`
    /// implementation.
    _marker: PhantomData<R::GuardMarker>,
}

unsafe impl<R: ScopedRawMutex + Send, T: ?Sized + Send> Send for BlockingMutex<R, T> {}
unsafe impl<R: ScopedRawMutex + Sync, T: ?Sized + Send> Sync for BlockingMutex<R, T> {}

#[cfg(feature = "std")]
#[inline(always)]
fn catch_unwind<F: FnOnce() -> R + std::panic::UnwindSafe, R>(
    f: F,
) -> Result<R, Box<dyn std::any::Any + Send>> {
    std::panic::catch_unwind(f)
}

#[cfg(not(feature = "std"))]
#[inline(always)]
fn catch_unwind<F: FnOnce() -> R, R>(f: F) -> Result<R, ()> {
    Ok(f())
}

#[cfg(feature = "std")]
#[inline(always)]
fn resume_unwind(payload: Box<dyn std::any::Any + Send>) -> ! {
    std::panic::resume_unwind(payload)
}

#[cfg(not(feature = "std"))]
#[inline(always)]
fn resume_unwind(_payload: ()) -> ! {
    unreachable!()
}

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

impl<R: ScopedRawMutex, T: ?Sized> BlockingMutex<R, T> {
    /// Locks the raw mutex and grants temporary access to the inner data
    ///
    /// Behavior when the lock is already locked is dependent on the behavior
    /// of the Raw mutex. See [`ScopedRawMutex::with_lock()`]'s documentation for
    /// more details
    pub fn with_lock<U>(&self, f: impl FnOnce(&mut T) -> U) -> U {
        let res = self.raw.with_lock(|| {
            let ptr = self.data.get();
            // SAFETY: Raw Mutex proves we have exclusive access to the inner data
            let inner = unsafe { &mut *ptr };
            catch_unwind(AssertUnwindSafe(|| f(inner)))
        });
        match res {
            Ok(g) => g,
            Err(b) => resume_unwind(b),
        }
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

impl<R: RawMutex, T: ?Sized> BlockingMutex<R, T> {
    /// Locks the raw mutex, returning a [`MutexGuard`] that grants temporary
    /// access to the inner data.
    ///
    /// Behavior when the lock is already locked is dependent on the behavior
    /// of the raw mutex. See [`RawMutex::lock()`]'s documentation for
    /// more details
    ///
    /// This method is only available when the `R` type parameter implements the
    /// [`RawMutex`] trait. If `R` can only implement the [`ScopedRawMutex`]
    /// subset, consider [`BlockingMutex::with_lock()`] instead.
    pub fn lock(&self) -> MutexGuard<'_, R, T> {
        self.raw.lock();
        MutexGuard {
            lock: self,
            _marker: PhantomData,
        }
    }

    /// Attempts to lock the raw mutex, returning a [`MutexGuard`] that grants
    /// temporary access to the inner data if the lock can be acquired.
    ///
    /// This method will never block, and instead returns [`None`] immediately
    /// if the mutex is already locked. To block until the mutex can be
    /// acquired, use [`BlockingMutex::lock()`] instead.
    ///
    /// This method is only available when the `R` type parameter implements the
    /// [`RawMutex`] trait. If `R` can only implement the [`ScopedRawMutex`]
    /// subset, consider [`BlockingMutex::try_with_lock()`] instead.
    ///
    /// # Returns
    ///
    /// - [`Some`]`(`[`MutexGuard`]`<R, T>)` if the mutex was not already
    ///   locked.
    /// - [`None`] if the mutex is already locked.
    pub fn try_lock(&self) -> Option<MutexGuard<'_, R, T>> {
        if self.raw.try_lock() {
            Some(MutexGuard {
                lock: self,
                _marker: PhantomData,
            })
        } else {
            None
        }
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

#[cfg(feature = "fmt")]
impl<R, T> core::fmt::Debug for BlockingMutex<R, T>
where
    R: ScopedRawMutex + core::fmt::Debug,
    T: ?Sized + core::fmt::Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut s = f.debug_struct("BlockingMutex");
        s.field("raw", &self.raw);

        self.try_with_lock(|data| s.field("data", &data).finish())
            .unwrap_or_else(|| s.field("data", &format_args!("<locked>")).finish())
    }
}

// === impl MutexGuard ===

impl<R: RawMutex, T: ?Sized> Drop for MutexGuard<'_, R, T> {
    fn drop(&mut self) {
        debug_assert!(
            self.lock.raw.is_locked(),
            "tried to unlock a `Mutex` that was not locked! this is almost \
             certainly a bug in the `RawMutex` implementation (`{}`)",
            core::any::type_name::<R>(),
        );
        unsafe {
            // SAFETY: a `MutexGuard` is only created when the lock has
            // been acquired, so we are allowed to unlock it.
            self.lock.raw.unlock();
        }
    }
}

impl<R: RawMutex, T: ?Sized> Deref for MutexGuard<'_, R, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        debug_assert!(
            self.lock.raw.is_locked(),
            "tried to dereference a `MutexGuard` that was not locked! this is \
             almost certainly a bug in the `RawMutex` implementation (`{}`)",
            core::any::type_name::<R>(),
        );
        unsafe {
            // SAFETY: a `MutexGuard` should only be constructed once the lock
            // is locked, and the lock should not be unlocked until the guard is
            // dropped.
            &*self.lock.data.get()
        }
    }
}

impl<R: RawMutex, T: ?Sized> DerefMut for MutexGuard<'_, R, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        debug_assert!(
            self.lock.raw.is_locked(),
            "tried to mutably dereference a `MutexGuard` that was not locked! \
             this is almost certainly a bug in the `RawMutex` implementation \
             (`{}`)",
            core::any::type_name::<R>(),
        );
        unsafe {
            // SAFETY: a `MutexGuard` should only be constructed once the lock
            // is locked, and the lock should not be unlocked until the guard is
            // dropped.
            &mut *self.lock.data.get()
        }
    }
}

unsafe impl<R, T> Send for MutexGuard<'_, R, T>
where
    // A `MutexGuard` can only be `Send` if the protected data is `Send`, because
    // owning the guard can be used to move the data out of the lock using
    // `mem::replace` or similar.
    T: ?Sized + Send,
    // This is just required by the bounds on the declaration of `MutexGuard`:
    R: RawMutex,
    // The guard marker must be `Send` to allow sending the guard to another
    // thread/core.
    R::GuardMarker: Send,
{
}
unsafe impl<R, T> Sync for MutexGuard<'_, R, T>
where
    // An `&`-reference to a `MutexGuard` is morally equivalent to an
    // `&`-reference to a `T`.
    T: ?Sized + Sync,
    // This is just required by the bounds on the declaration of `MutexGuard`:
    R: RawMutex,
{
}

#[cfg(feature = "fmt")]
impl<R, T> core::fmt::Debug for MutexGuard<'_, R, T>
where
    T: ?Sized + core::fmt::Debug,
    R: RawMutex,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(&self.data, f)
    }
}

#[cfg(feature = "fmt")]
impl<R, T> core::fmt::Display for MutexGuard<'_, R, T>
where
    T: ?Sized + core::fmt::Display,
    R: RawMutex,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(&self.data, f)
    }
}

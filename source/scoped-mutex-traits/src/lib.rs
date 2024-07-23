//! Scoped Mutex Traits
//!
//! This crate provides traits that are aimed at compatibility for scoped mutexes.
//!
//! Compared to the more general traits provided by the [`lock_api`] crate, these traits
//! are aimed at being more compatible with implementations based on critical sections,
//! are easier to work with in a nested or strictly LIFO pattern.
//!
//! [`lock_api`]: https://docs.rs/lock_api/
#![deny(missing_docs)]
#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

/// Const Init Trait
///
/// This trait is intended for use when implementers of [`ScopedRawMutex`] that can
/// be constructed in const context, e.g. for placing in a `static`
pub trait ConstInit {
    /// Create a new instance.
    ///
    /// This is a const instead of a method to allow creating instances in const context.
    const INIT: Self;
}

/// Raw mutex trait.
///
/// This mutex is "raw", which means it does not actually contain the protected data, it
/// just implements the mutex mechanism. For most uses you should use `BlockingMutex`
/// from the `scoped-mutex-impls` crate instead, which is generic over a
/// `ScopedRawMutex` and contains the protected data.
///
/// # Safety
///
/// ScopedRawMutex implementations must ensure that, while locked, no other thread can lock
/// the RawMutex concurrently. This can usually be implemented using an [`AtomicBool`]
/// to track the "taken" state. See the `scoped-mutex-impls` crate for examples of
/// correct implementations.
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
    fn try_with_lock<R>(&self, f: impl FnOnce() -> R) -> Option<R>;

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
    fn with_lock<R>(&self, f: impl FnOnce() -> R) -> R;

    /// Is this mutex currently locked?
    fn is_locked(&self) -> bool;
}

/// Raw RAII mutex trait.
///
/// # Safety
///
/// Implementations of this trait must ensure that the mutex is actually
/// exclusive: a lock can't be acquired while the mutex is already locked.
pub unsafe trait RawMutex {
    /// Marker type which determines whether a lock guard should be [`Send`].
    type GuardMarker;

    /// Acquires this mutex, blocking the current thread/CPU core until it is
    /// able to do so.
    fn lock(&self);

    /// Attempts to acquire this mutex without blocking. Returns `true`
    /// if the lock was successfully acquired and `false` otherwise.
    fn try_lock(&self) -> bool;

    /// Unlocks this mutex.
    ///
    /// # Safety
    ///
    /// This method may only be called if the mutex is held in the current
    /// context, i.e. it must be paired with a successful call to [`lock`] or
    /// [`try_lock`].
    ///
    /// [`lock`]: RawMutex::lock
    /// [`try_lock`]: RawMutex::try_lock
    unsafe fn unlock(&self);

    /// Returns `true` if the mutex is currently locked.
    fn is_locked(&self) -> bool;
}

unsafe impl<M: RawMutex> ScopedRawMutex for M {
    #[must_use]
    fn try_with_lock<R>(&self, f: impl FnOnce() -> R) -> Option<R> {
        if self.try_lock() {
            // Using a drop guard ensures that the mutex is unlocked when this
            // function exits, even if `f()` panics.
            let _unlock = Unlock(self);
            Some(f())
        } else {
            None
        }
    }

    fn with_lock<R>(&self, f: impl FnOnce() -> R) -> R {
        self.lock();
        // Using a drop guard ensures that the mutex is unlocked when this
        // function exits, even if `f()` panics.
        let _unlock = Unlock(self);
        f()
    }

    /// Is this mutex currently locked?
    fn is_locked(&self) -> bool {
        RawMutex::is_locked(self)
    }

}

/// Implementation detail of the `ScopedRawMutex` implementation for `RawMutex`.
/// This is a drop guard that unlocks the `RawMutex` when it's dropped. This is
/// used to ensure that the `RawMutex` is always unlocked when the
/// `ScopedRawMutex::with_lock` or `ScopedRawMutex::try_with_lock` closures are
/// exited, even if they are exited by a panic rather than by a normal return.
struct Unlock<'mutex, M: RawMutex>(&'mutex M);

impl<M: RawMutex> Drop for Unlock<'_, M> {
    fn drop(&mut self) {
        unsafe {
            // Safety: Constructing an `Unlock` is only safe if the mutex has
            // been locked. Callers are responsible for ensuring this invariant;
            // since this struct is only constructed in this module, we do so.
            self.0.unlock()
        }
    }
}

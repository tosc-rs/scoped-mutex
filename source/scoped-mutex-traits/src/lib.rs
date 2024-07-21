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

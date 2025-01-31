# mutex

> When a mutex and a closure love each other very much.

[![Crates.io Version](https://img.shields.io/crates/v/mutex)][crates-link]
[![Crates.io License](https://img.shields.io/crates/l/mutex)][license-link]
[![docs.rs](https://img.shields.io/docsrs/mutex)][docsrs-link]
[![GitHub Release]][release-link]
[![CI]][ci-link]

[crates-link]: https://crates.io/crates/mutex
[license-link]: https://github.com/tosc-rs/scoped-mutex?tab=readme-ov-file#license
[docsrs-link]: https://docs.rs/mutex
[release-link]:
    https://github.com/tosc-rs/scoped-mutex/releases?q=main-*&expanded=true
[ci-link]: https://github.com/tosc-rs/scoped-mutex/actions/workflows/ci.yml
[CI]: https://github.com/tosc-rs/scoped-mutex/actions/workflows/ci.yml/badge.svg
[GitHub Release]: https://img.shields.io/github/v/release/tosc-rs/scoped-mutex?sort=date&filter=main-*&display_name=tag

Mutex implementations using [`mutex-traits`].

## Versioning, and which crate to use?

The [`mutex-traits`] crate should be used by **library crates** that want to be generic
over different ways of exclusive access.

The [`mutex`] crate should be used by **applications** that need to select which implementation
is appropriate for their use case. The [`mutex`] crate also re-exports the [`mutex-traits`]
crate for convenience, so applications only need to pull in one direct dependency.

While both crates are >= 1.0, it should be expected that it is more likely that [`mutex`] crate
will make breaking changes someday. The hope is that [`mutex-traits`] NEVER releases a 2.0
version, which means that even if there are 1.x, 2.x, 3.x, etc. versions of the [`mutex`] crate,
they all can be used interchangably since they implement the 1.x [`mutex-traits`] interfaces.

If you are a library crate, consider ONLY relying on the [`mutex-traits`] crate directly, and
put any use of the [`mutex`] crate behind a feature flag or in the `dev-dependencies` section.

## Crate Feature Flags

The following feature flags enable implementations of
[`mutex_traits::ScopedRawMutex`][`ScopedRawMutex`] and
[`mutex_traits::RawMutex`][`RawMutex`]:

+ **`impl-critical-section` (default: `true`)**: Enables implementations of
  [`ScopedRawMutex`] for the [`critical-section`] crate.
+ **`impl-lock_api-0_4` (default: `false`)**: Enables a wrapper type
  implementing [`RawMutex`] for types implementing the [`lock_api`]  crate's
  [`RawMutex` trait][lock_api::RawMutex].
+ **`impl-unsafe-cortex-m-single-core` (default: `false`)**: Enables
  implementations of [`ScopedRawMutex`] which may only be used on single-core
  Cortex-M devices.

In addition, this crate exposes the following additional feature flags,  for
functionality other than implementations of [`ScopedRawMutex`]/[`RawMutex`]:

+ **`std` (default: `false`)**: Enables features that require the Rust standard
  library.

  When this feature flag is disabled, this crate compiles with
  `#![no_std]` and does not require `liballoc`.
+ **`fmt` (default: `false`)**: Enables implementations of `core::fmt::Debug`
  and `core::fmt::Display` for types provided by this crate.

  These formatting trait impls are feature-flagged so that they can  be disabled
  by embedded projects and other use-cases where minimizing binary size is
  important.

[`mutex`]: https://crates.io/crates/mutex
[`mutex-traits`]: https://crates.io/crates/mutex-traits
[`critical-section`]: https://crates.io/crates/critical-section
[`lock_api`]: https://crates.io/crates/critical-section
[`ScopedRawMutex`]:
    https://docs.rs/mutex-traits/latest/mutex_traits/trait.ScopedRawMutex.html
[`RawMutex`]:
    https://docs.rs/mutex-traits/latest/mutex_traits/trait.RawMutex.html
[lock_api::RawMutex]:
    https://docs.rs/lock_api/latest/lock_api/trait.RawMutex.html

## Provenance

Portions of this code are forked from the `embassy-sync` crate.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

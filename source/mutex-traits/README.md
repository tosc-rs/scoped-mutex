# mutex

> When a mutex and a closure love each other very much.

[![Crates.io Version](https://img.shields.io/crates/v/mutex-traits)][crates-link]
[![Crates.io License](https://img.shields.io/crates/l/mutex-traits)][license-link]
[![docs.rs](https://img.shields.io/docsrs/mutex-traits)][docsrs-link]
[![GitHub Release]][release-link]
[![CI]][ci-link]

[crates-link]: https://crates.io/crates/mutex-traits
[license-link]: https://github.com/tosc-rs/scoped-mutex?tab=readme-ov-file#license
[docsrs-link]: https://docs.rs/mutex-traits
[release-link]:
    https://github.com/tosc-rs/scoped-mutex/releases?q=traits-*&expanded=true
[ci-link]: https://github.com/tosc-rs/scoped-mutex/actions/workflows/ci.yml
[CI]: https://github.com/tosc-rs/scoped-mutex/actions/workflows/ci.yml/badge.svg
[GitHub Release]: https://img.shields.io/github/v/release/tosc-rs/scoped-mutex?sort=date&filter=traits-*&display_name=tag

Traits abstracting over mutex implementations.

Compared to the more general traits provided by the [`lock_api`] crate, these
traits  are aimed at being more compatible with implementations based on
critical sections, are easier to work with in a nested or strictly LIFO pattern.

## Versioning, and which crate to use?

The [`mutex-traits`] crate should be used by **library crates** that want to be generic
over different ways of exclusive access.

The [`mutex`] crate should be used by **applications** that need to select which implementation
is appropriate for their use case. The [`mutex`] crate also re-exports the [`mutex-traits`]
crate for convenience, so applications only need to pull in one direct dependency.

While both crates are >= 1.0, it should be expected that it is **MUCH** more likely that [`mutex`]
crate will make breaking changes someday. The hope is that [`mutex-traits`] NEVER releases a 2.0
version, which means that even if there are 1.x, 2.x, 3.x, etc. versions of the [`mutex`] crate,
they all can be used interchangably since they implement the 1.x [`mutex-traits`] interfaces.

If you are a library crate, consider ONLY relying on the [`mutex-traits`] crate directly, and
put any use of the [`mutex`] crate behind a feature flag or in the `dev-dependencies` section.

## Provenance

Portions of this code are forked from the `embassy-sync` crate.

The `RawMutex` trait is adapted from the trait of the same name in the
[`lock_api`] crate, by Amanieu d'Antras.

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

[`mutex`]: https://crates.io/crates/mutex
[`mutex-traits`]: https://crates.io/crates/mutex-traits
[`lock_api`]: https://docs.rs/lock_api/

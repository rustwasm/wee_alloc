# Contributing to `wee_alloc`

Hi! We'd love to have your contributions! If you want help or mentorship, reach
out to us in a GitHub issue, or ping `fitzgen` in [`#rust-wasm` on
`irc.mozilla.org`](irc://irc.mozilla.org#rust-wasm) and introduce yourself.

<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->


- [Code of Conduct](#code-of-conduct)
- [Building and Testing](#building-and-testing)
  - [Prerequisites](#prerequisites)
  - [Type Checking](#type-checking)
  - [Building](#building)
  - [Testing](#testing)
- [Automatic code formatting](#automatic-code-formatting)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

## Code of Conduct

We abide by the [Rust Code of Conduct][coc] and ask that you do as well.

[coc]: https://www.rust-lang.org/en-US/conduct.html

## Building and Testing

### Prerequisites

Ensure you have the `wasm32-unknown-unknown` target installed with `rustup`:

```
$ rustup update
$ rustup target add wasm32-unknown-unknown --toolchain nightly
```

Ensure that you have `wasm-gc` installed:

```
$ cargo install --git https://github.com/alexcrichton/wasm-gc
```

Ensure that you have `cargo-readme` installed:

```
$ cargo install cargo-readme
```

Finally, ensure that you have [`binaryen`'s
`wasm-opt`](https://github.com/WebAssembly/binaryen) installed.

### Type Checking

The `check.sh` script essentially runs `cargo check` in each crate with all the
various features and targets.

```
$ ./check.sh
```

### Building

The `build.sh` script essentially runs `cargo build` in each crate with all the
various features and targets.

```
$ ./build.sh
```

### Testing

The `test.sh` script essentially runs `cargo test` in each crate with all the
various features and targets.

```
$ ./test.sh
```

## Automatic code formatting

We use [`rustfmt`](https://github.com/rust-lang-nursery/rustfmt) to enforce a
consistent code style across the whole code base.

You can install the latest version of `rustfmt` with this command:

```
$ rustup update
$ rustup component add rls-preview
```

Ensure that `~/.rustup/toolchains/$YOUR_HOST_TARGET/bin/` is on your `$PATH`.

Once that is taken care of, you can (re)format all code by running this command
from the root of the repository:

```
$ cargo fmt --all
```

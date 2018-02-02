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

## Automatic Code Formatting

We use [`rustfmt`](https://github.com/rust-lang-nursery/rustfmt) to enforce a
consistent code style across the whole code base.

You can install the latest version of `rustfmt` with this command:

```
$ rustup update
$ rustup component add rustfmt-preview
```

Ensure that `~/.rustup/toolchains/$YOUR_HOST_TARGET/bin/` is on your `$PATH`.

Once that is taken care of, you can (re)format all code by running this command
from the root of the repository:

```
$ cargo fmt --all
```

## Pull Requests

All pull requests must be reviewed and approved of by at least one [team](#team)
member before merging. See [Contributions We Want](#contributions-we-want) for
details on what should be included in what kind of pull request.

## Contributions We Want

* **Bug fixes!** Include a regression test.

* **Code size reductions!** Include before and after `.wasm` sizes (as reported
  by `build.sh`) in your commit or pull request message.

* **Performance improvements!** Include before and after `#[bench]` numbers, or
  write a new `#[bench]` that exercises the code path, if none exists already.

If you make two of these kinds of contributions, you should seriously consider
joining our [team](#team)!

Where we need help:

* Issues labeled ["help wanted"][help-wanted] are issues where we could use a
  little help from you.

* Issues labeled ["mentored"][mentored] are issues that don't really involve any
  more investigation, just implementation. We've outlined what needs to be done,
  and a team_ member has volunteered to help whoever claims the issue implement
  it, and get the implementation merged.

* Issues labeled ["good first issue"][gfi] are issues where fixing them would be
  a great introduction to the code base.

[help-wanted]: https://github.com/fitzgen/wee_alloc/labels/help%20wanted

[mentored]: https://github.com/fitzgen/wee_alloc/labels/mentored

[gfi]: https://github.com/fitzgen/wee_alloc/labels/good%20first%20issue

## Team

* `fitzgen`

Larger, more nuanced decisions about design, architecture, breaking changes,
trade offs, etc are made by team consensus. In other words, decisions on things
that aren't straightforward improvements or bug fixes to things that already
exist in `wee_alloc`. If consensus can't be made, then `fitzgen` has the last
word.

**We need more team members!**
[Drop a comment on this issue if you are interested in joining.][join]

[join]: https://github.com/fitzgen/wee_alloc/issues/6

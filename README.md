<div align="center">

  <h1><code>wee_alloc</code></h1>

  <strong>The <u>W</u>asm-<u>E</u>nabled, <u>E</u>lfin Allocator</strong>

  <p>
    <a href="https://travis-ci.org/rustwasm/wee_alloc"><img src="https://img.shields.io/travis/rustwasm/wee_alloc.svg?style=flat-square" alt="Build Status" /></a>
    <a href="https://ci.appveyor.com/project/fitzgen/wee-alloc/branch/master"><img src="https://img.shields.io/travis/rustwasm/wee_alloc.svg?style=flat-square" alt="Build Status" /></a>
    <a href="https://crates.io/crates/wee_alloc"><img src="https://img.shields.io/crates/v/wee_alloc.svg?style=flat-square" alt="Crates.io version" /></a>
    <a href="https://crates.io/crates/wee_alloc"><img src="https://img.shields.io/crates/d/wee_alloc.svg?style=flat-square" alt="Download" /></a>
    <a href="https://docs.rs/wee_alloc"><img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square" alt="docs.rs docs" /></a>
  </p>

  <h3>
    <a href="https://docs.rs/wee_alloc">API Docs</a>
    <span> | </span>
    <a href="https://github.com/rustwasm/wee_alloc/blob/master/CONTRIBUTING.md">Contributing</a>
    <span> | </span>
    <a href="https://discordapp.com/channels/442252698964721669/443151097398296587">Chat</a>
  </h3>

  <sub>Built with 🦀🕸 by <a href="https://rustwasm.github.io/">The Rust and WebAssembly Working Group</a></sub>
</div>


### About

`wee_alloc`: The **W**asm-**E**nabled, **E**lfin Allocator.

- **Elfin, i.e. small:** Generates less than a kilobyte of uncompressed
  WebAssembly code. Doesn't pull in the heavy panicking or formatting
  infrastructure. `wee_alloc` won't bloat your `.wasm` download size on the Web.

- **WebAssembly enabled:** Designed for the `wasm32-unknown-unknown` target and
  `#![no_std]`.

`wee_alloc` is focused on targeting WebAssembly, producing a small `.wasm` code
size, and having a simple, correct implementation. It is geared towards code
that makes a handful of initial dynamically sized allocations, and then performs
its heavy lifting without any further allocations. This scenario requires *some*
allocator to exist, but we are more than happy to trade allocation performance
for small code size. In contrast, `wee_alloc` would be a poor choice for a
scenario where allocation is a performance bottleneck.

Although WebAssembly is the primary target, `wee_alloc` also has an `mmap` based
implementation for unix systems, a `VirtualAlloc` implementation for Windows,
and a static array-based backend for OS-independent environments. This enables
testing `wee_alloc`, and code using `wee_alloc`, without a browser or
WebAssembly engine.

`wee_alloc` compiles on stable Rust 1.33 and newer.

- [Using `wee_alloc` as the Global Allocator](#using-wee_alloc-as-the-global-allocator)
- [`cargo` Features](#cargo-features)
- [Implementation Notes and Constraints](#implementation-notes-and-constraints)
- [License](#license)
- [Contribution](#contribution)

### Using `wee_alloc` as the Global Allocator

```rust
extern crate wee_alloc;

// Use `wee_alloc` as the global allocator.
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;
```

### `cargo` Features

- **size_classes**: On by default. Use size classes for smaller allocations to
  provide amortized *O(1)* allocation for them. Increases uncompressed `.wasm`
  code size by about 450 bytes (up to a total of ~1.2K).

- **extra_assertions**: Enable various extra, expensive integrity assertions and
  defensive mechanisms, such as poisoning freed memory. This incurs a large
  runtime overhead. It is useful when debugging a use-after-free or `wee_alloc`
  itself.

- **static_array_backend**: Force the use of an OS-independent backing
  implementation with a global maximum size fixed at compile time.  Suitable for
  deploying to non-WASM/Unix/Windows `#![no_std]` environments, such as on
  embedded devices with esoteric or effectively absent operating systems. The
  size defaults to 32 MiB (33554432 bytes), and may be controlled at build-time
  by supplying an optional environment variable to cargo,
  `WEE_ALLOC_STATIC_ARRAY_BACKEND_BYTES`. Note that this feature requires
  nightly Rust.

- **nightly**: Enable usage of nightly-only Rust features, such as implementing
  the `Alloc` trait (not to be confused with the stable `GlobalAlloc` trait!)

### Implementation Notes and Constraints

- `wee_alloc` imposes two words of overhead on each allocation for maintaining
  its internal free lists.

- Deallocation is an *O(1)* operation.

- `wee_alloc` will never return freed pages to the WebAssembly engine /
  operating system. Currently, WebAssembly can only grow its heap, and can never
  shrink it. All allocated pages are indefinitely kept in `wee_alloc`'s internal
  free lists for potential future allocations, even when running on unix
  targets.

- `wee_alloc` uses a simple, first-fit free list implementation. This means that
  allocation is an *O(n)* operation.

  Using the `size_classes` feature enables extra free lists dedicated to small
  allocations (less than or equal to 256 words). The size classes' free lists
  are populated by allocating large blocks from the main free list, providing
  amortized *O(1)* allocation time. Allocating from the size classes' free lists
  uses the same first-fit routines that allocating from the main free list does,
  which avoids introducing more code bloat than necessary.

Finally, here is a diagram giving an overview of `wee_alloc`'s implementation:

```
+------------------------------------------------------------------------------+
| WebAssembly Engine / Operating System                                        |
+------------------------------------------------------------------------------+
                   |
                   |
                   | 64KiB Pages
                   |
                   V
+------------------------------------------------------------------------------+
| Main Free List                                                               |
|                                                                              |
|          +------+     +------+     +------+     +------+                     |
| Head --> | Cell | --> | Cell | --> | Cell | --> | Cell | --> ...             |
|          +------+     +------+     +------+     +------+                     |
|                                                                              |
+------------------------------------------------------------------------------+
                   |                                    |            ^
                   |                                    |            |
                   | Large Blocks                       |            |
                   |                                    |            |
                   V                                    |            |
+---------------------------------------------+         |            |
| Size Classes                                |         |            |
|                                             |         |            |
|             +------+     +------+           |         |            |
| Head(1) --> | Cell | --> | Cell | --> ...   |         |            |
|             +------+     +------+           |         |            |
|                                             |         |            |
|             +------+     +------+           |         |            |
| Head(2) --> | Cell | --> | Cell | --> ...   |         |            |
|             +------+     +------+           |         |            |
|                                             |         |            |
| ...                                         |         |            |
|                                             |         |            |
|               +------+     +------+         |         |            |
| Head(256) --> | Cell | --> | Cell | --> ... |         |            |
|               +------+     +------+         |         |            |
|                                             |         |            |
+---------------------------------------------+         |            |
                      |            ^                    |            |
                      |            |                    |            |
          Small       |      Small |        Large       |      Large |
          Allocations |      Frees |        Allocations |      Frees |
                      |            |                    |            |
                      |            |                    |            |
                      |            |                    |            |
                      |            |                    |            |
                      |            |                    |            |
                      V            |                    V            |
+------------------------------------------------------------------------------+
| User Application                                                             |
+------------------------------------------------------------------------------+
```

### License

Licensed under the [Mozilla Public License 2.0](https://www.mozilla.org/en-US/MPL/2.0/).

[TL;DR?](https://choosealicense.com/licenses/mpl-2.0/)

> Permissions of this weak copyleft license are conditioned on making available
> source code of licensed files and modifications of those files under the same
> license (or in certain cases, one of the GNU licenses). Copyright and license
> notices must be preserved. Contributors provide an express grant of patent
> rights. However, a larger work using the licensed work may be distributed
> under different terms and without source code for files added in the larger
> work.

### Contribution

See
[CONTRIBUTING.md](https://github.com/rustwasm/wee_alloc/blob/master/CONTRIBUTING.md)
for hacking!


# `wee_alloc`

[![](https://docs.rs/wee_alloc/badge.svg)](https://docs.rs/wee_alloc/)
[![](https://img.shields.io/crates/v/wee_alloc.svg)](https://crates.io/crates/wee_alloc)
[![](https://img.shields.io/crates/d/wee_alloc.svg)](https://crates.io/crates/wee_alloc)
[![Travis CI Build Status](https://travis-ci.org/rustwasm/wee_alloc.svg?branch=master)](https://travis-ci.org/rustwasm/wee_alloc)
[![AppVeyor Build status](https://ci.appveyor.com/api/projects/status/bqh8elm9wy0k5x2r/branch/master?svg=true)](https://ci.appveyor.com/project/rustwasm/wee-alloc/branch/master)

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
implementation for unix systems and a `VirtualAlloc` implementation for Windows.
This enables testing `wee_alloc`, and code using `wee_alloc`, without a browser
or WebAssembly engine.

**⚠ Custom allocators currently require Nightly Rust. ⚠**

- [Using `wee_alloc` as the Global Allocator](#using-wee_alloc-as-the-global-allocator)
  - [With `#![no_std]`](#with-no_std)
  - [With `std`](#with-std)
- [`cargo` Features](#cargo-features)
- [Implementation Notes and Constraints](#implementation-notes-and-constraints)
- [License](#license)
- [Contribution](#contribution)

### Using `wee_alloc` as the Global Allocator

To get the smallest `.wasm` sizes, you want to use `#![no_std]` with a custom
panicking hook that avoids using any of the `core::fmt`
infrastructure. Nevertheless, `wee_alloc` is also usable with `std`.

#### With `#![no_std]`

```rust
// We aren't using the standard library.
#![no_std]

// Required to use the `alloc` crate and its types, the `abort` intrinsic, and a
// custom panic handler.
#![feature(alloc, core_intrinsics, panic_implementation, lang_items)]

extern crate alloc;
extern crate wee_alloc;

// Use `wee_alloc` as the global allocator.
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

// Need to provide a tiny `panic` implementation for `#![no_std]`.
// This translates into an `unreachable` instruction that will
// raise a `trap` the WebAssembly execution if we panic at runtime.
#[panic_implementation]
#[no_mangle]
pub fn panic(_info: &::core::panic::PanicInfo) -> ! {
    unsafe {
        ::core::intrinsics::abort();
    }
}

// Need to provide a tiny `oom` lang-item implementation for
// `#![no_std]`.
#[lang = "oom"]
#[no_mangle]
pub extern "C" fn oom() -> ! {
    unsafe {
        ::core::intrinsics::abort();
    }
}

// And now you can use `alloc` types!
use alloc::arc::Arc;
use alloc::boxed::Box;
use alloc::vec::Vec;
// etc...
```

#### With `std`

```rust
// Required to replace the global allocator.
#![feature(global_allocator)]

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
  implementation with a global maximum size fixed at compile time.
  Suitable for deploying to non-WASM/Unix/Windows `#![no_std]` environments,
  such as on embedded devices with esoteric or effectively absent operating
  systems. The size defaults to 32 MiB (33554432 bytes), and may be controlled
  at build-time by supplying an optional environment variable to cargo,
  `WEE_ALLOC_STATIC_ARRAY_BACKEND_BYTES`

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


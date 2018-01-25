/*!

[![](https://docs.rs/wee_alloc/badge.svg)](https://docs.rs/wee_alloc/)
[![](https://img.shields.io/crates/v/wee_alloc.svg)](https://crates.io/crates/wee_alloc)
[![](https://img.shields.io/crates/d/wee_alloc.svg)](https://crates.io/crates/wee_alloc)
[![Build Status](https://travis-ci.org/fitzgen/wee_alloc.svg?branch=master)](https://travis-ci.org/fitzgen/wee_alloc)

`wee_alloc`: The **W**asm-**E**nabled, **E**lfin Allocator.

- **Elfin, i.e. small:** Generates less than a kilobyte of uncompressed
  WebAssembly code. `wee_alloc` won't bloat your `.wasm` download size on the
  Web.

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
implementation for unix systems. This enables testing `wee_alloc`, and code
using `wee_alloc`, without a browser or WebAssembly engine.

- [Using `wee_alloc` as the Global Allocator](#using-wee_alloc-as-the-global-allocator)
  - [With `#![no_std]`](#with-no_std)
  - [With `std`](#with-std)
- [`cargo` Features](#cargo-features)
- [Implementation Notes and Constraints](#implementation-notes-and-constraints)
- [License](#license)
- [Contribution](#contribution)

## Using `wee_alloc` as the Global Allocator

To get the smallest `.wasm` sizes, you want to use `#![no_std]` with a custom
panicking hook that avoids using any of the `core::fmt`
infrastructure. Nevertheless, `wee_alloc` is also usable with `std`.

### With `#![no_std]`

```
// We aren't using the standard library.
#![no_std]

// Required to replace the global allocator.
#![feature(global_allocator)]

// Required to use the `alloc` crate and its types, the `abort` intrinsic, and a
// custom panic handler.
#![feature(alloc, core_intrinsics, lang_items)]

extern crate alloc;
extern crate wee_alloc;

// Use `wee_alloc` as the global allocator.
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

// Need to provide a tiny `panic_fmt` lang-item implementation for `#![no_std]`.
// This implementation will translate panics into traps in the resulting
// WebAssembly.
#[lang = "panic_fmt"]
extern "C" fn panic_fmt(
    _args: ::core::fmt::Arguments,
    _file: &'static str,
    _line: u32
) -> ! {
    use core::intrinsics;
    unsafe {
        intrinsics::abort();
    }
}

// And now you can use `alloc` types!
use alloc::arc::Arc;
use alloc::boxed::Box;
use alloc::vec::Vec;
// etc...
```

### With `std`

```
// Required to replace the global allocator.
#![feature(global_allocator)]

extern crate wee_alloc;

// Use `wee_alloc` as the global allocator.
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;
```

## `cargo` Features

- **size_classes**: On by default. Use size classes for smaller allocations to
  provide amortized *O(1)* allocation for them. Increases uncompressed `.wasm`
  code size by about 400 bytes (up to a total ~950 bytes).

- **extra_assertions**: Enable various extra, expensive integrity assertions and
  defensive mechanisms, such as poisoning freed memory. This incurs a large
  runtime overhead. It is useful when debugging a use-after-free or `wee_alloc`
  itself.

## Implementation Notes and Constraints

- `wee_alloc` imposes two words of overhead on each allocation for maintaining
  its internal free lists.

- The maximum alignment supported is word alignment.

- Deallocation always pushes to the front of the free list, making it an *O(1)*
  operation.

  The tradeoff is that physically adjacent cells `a` and `b` are never
  re-unified into a single cell `ab` so that they could service a potential
  allocation of size `n` where

      size(max(size(a), size(b))) < n <= size(a) + size(b)

  This leads to higher fragmentation and peak memory usage.

  It also follows that `wee_alloc` will never return freed pages to the
  WebAssembly engine / operating system. They are indefinitely kept in its
  internal free lists for potential future allocations. Once a WebAssembly
  module instance / process reaches peak page usage, its usage will remain at
  that peak until finished.

- `wee_alloc` uses a simple first-fit free list implementation. This means that
  allocation is an *O(n)* operation.

  Using the `size_classes` feature enables extra free lists dedicated to small
  allocations (less than or equal to 256 words). The size classes' free lists
  are populated by allocating large blocks from the main free list, providing
  amortized *O(1)* allocation time. Allocating from the size classes' free lists
  uses the same first-fit routines that allocating from the main free list does,
  which avoids introducing more code bloat than necessary.

Finally, here is a diagram giving an overview of how `wee_alloc` is implemented:

```text
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

## License

Licensed under the [Mozilla Public License 2.0](https://www.mozilla.org/en-US/MPL/2.0/).

[TL;DR?](https://choosealicense.com/licenses/mpl-2.0/)

> Permissions of this weak copyleft license are conditioned on making available
> source code of licensed files and modifications of those files under the same
> license (or in certain cases, one of the GNU licenses). Copyright and license
> notices must be preserved. Contributors provide an express grant of patent
> rights. However, a larger work using the licensed work may be distributed
> under different terms and without source code for files added in the larger
> work.

## Contribution

See
[CONTRIBUTING.md](https://github.com/fitzgen/wee_alloc/blob/master/CONTRIBUTING.md)
for hacking!

 */

// TODO:
// - new crate to expose posix `malloc` and `free`
// - test bootstrapping rustc with this as global allocator
// - graphviz visualization of free lists, statistics on fregmentation,
//   etc. behind a feature

#![no_std]
#![feature(alloc, allocator_api, core_intrinsics, global_allocator)]
#![cfg_attr(target_arch = "wasm32", feature(link_llvm_intrinsics))]

extern crate alloc;

#[cfg(all(unix, not(target_arch = "wasm32")))]
extern crate libc;

#[macro_use]
mod extra_assert;

mod const_init;

#[cfg(all(not(unix), not(target_arch = "wasm32")))]
compile_error! {
    "There is no `wee_alloc` implementation for this target; want to send a pull request? :)"
}

#[cfg_attr(target_arch = "wasm32",
           path = "./imp_wasm32.rs")]
#[cfg_attr(all(unix, not(target_arch = "wasm32")),
           path = "./imp_unix.rs")]
mod imp;

#[cfg(feature = "size_classes")]
mod size_classes;

pub mod units;

use alloc::heap::{Alloc, Layout, AllocErr};
use const_init::ConstInit;
use core::isize;
use core::marker::Sync;
use core::ptr;
use units::{Bytes, Pages, RoundUpTo, size_of, Words};

/// The WebAssembly page size, in bytes.
pub const PAGE_SIZE: Bytes = Bytes(65536);

extra_only! {
    fn assert_is_word_aligned<T>(ptr: *mut T) {
        use core::mem;
        assert_eq!(
            ptr as usize % mem::size_of::<usize>(),
            0,
            "{:p} is not word-aligned",
            ptr
        )
    }
}

#[repr(C)]
struct Cell {
    size: Bytes,
    next: *mut Cell,
}

impl Cell {
    #[inline]
    unsafe fn write_initial(size: Bytes, uninitialized: *mut Cell) {
        assert_is_word_aligned(uninitialized);
        ptr::write(uninitialized, Cell {
            size,
            next: ptr::null_mut(),
        });
    }

    #[inline]
    unsafe fn data(cell: *mut Cell) -> *mut u8 {
        extra_assert!(!cell.is_null());
        assert_local_cell_invariants(cell);
        cell.offset(1) as *mut u8
    }

    #[inline]
    unsafe fn set_next(link: &mut *mut Cell, next: *mut Cell) {
        assert_local_cell_invariants(next);
        *link = next;
    }

    #[inline]
    unsafe fn insert_into_free_list(head: &mut *mut Cell, cell: *mut Cell, policy: &AllocPolicy) {
        assert_local_cell_invariants(*head);
        assert_local_cell_invariants(cell);

        write_free_pattern(cell, policy);
        Cell::set_next(&mut (*cell).next, *head);
        Cell::set_next(head, cell);

        assert_is_valid_free_list(*head, policy);
    }
}

#[cfg(feature = "extra_assertions")]
impl Cell {
    // Written to `Cell::next` when the `Cell` is allocated from a size class,
    // and removed from its free list. When the `Cell` is freed and returned to
    // its free list, then the value is overwritten with the proper free list
    // link.
    #[cfg(feature = "size_classes")]
    const SIZE_CLASS_ALLOCATED_NEXT: *mut Cell = 0x11111111 as *mut Cell;

    // Same thing as above, but for no-size-class/large allocations.
    const LARGE_ALLOCATED_NEXT: *mut Cell = 0x99999999 as *mut Cell;

    // Whenever a `Cell` is inserted into a size class's free list (either
    // because it was just freed or because it was freshly allocated from some
    // upstream source), we write this pattern over the `Cell`'s data.
    //
    // If you see unexpected `0x35353535` values, then either (a) you have a
    // use-after-free, or (b) there is a bug in `wee_alloc` and its size classes
    // implementation.
    #[cfg(feature = "size_classes")]
    const SIZE_CLASS_FREE_PATTERN: u8 = 0x35;

    // Same thing as above, but for data inside the no-size-class/large
    // allocations free list.
    //
    // If you see unexpected `0x57575757` values, then either (a) you have a
    // use-after-free, or (b) there is a bug in `wee_alloc` and its size classes
    // implementation.
    const LARGE_FREE_PATTERN: u8 = 0x57;

    #[cfg(feature = "size_classes")]
    fn is_allocated(cell: *mut Cell) -> bool {
        cell == Cell::SIZE_CLASS_ALLOCATED_NEXT || cell == Cell::LARGE_ALLOCATED_NEXT
    }

    #[cfg(not(feature = "size_classes"))]
    fn is_allocated(cell: *mut Cell) -> bool {
        cell == Cell::LARGE_ALLOCATED_NEXT
    }
}

extra_only! {
    fn set_allocated(cell: *mut Cell, policy: &AllocPolicy) {
        unsafe {
            extra_assert!(!cell.is_null());
            assert_is_word_aligned(cell);
            (*cell).next = policy.allocated_sentinel();
        }
    }
}

extra_only! {
    fn assert_is_allocated(cell: *mut Cell, policy: &AllocPolicy) {
        unsafe {
            extra_assert!(!cell.is_null());
            assert_is_word_aligned(cell);
            extra_assert_eq!(
                (*cell).next,
                policy.allocated_sentinel(),
                "cell {:p} should have next=allocated ({:p}) but found next={:p}",
                cell,
                policy.allocated_sentinel(),
                (*cell).next
            );
        }
    }
}

extra_only! {
    fn write_free_pattern(cell: *mut Cell, policy: &AllocPolicy) {
        unsafe {
            extra_assert!(!cell.is_null());
            assert_is_word_aligned(cell);
            ptr::write_bytes(Cell::data(cell), policy.free_pattern(), (*cell).size.0);
        }
    }
}

extra_only! {
    fn assert_is_poisoned_with_free_pattern(cell: *mut Cell, policy: &AllocPolicy) {
        use core::slice;
        unsafe {
            extra_assert!(!cell.is_null());
            assert_is_word_aligned(cell);
            let data = slice::from_raw_parts(Cell::data(cell) as *const u8, (*cell).size.0);
            let pattern = policy.free_pattern();
            extra_assert!(data.iter().all(|byte| *byte == pattern));
        }
    }
}

extra_only! {
    fn assert_next_is_not_in_data(cell: *mut Cell) {
        unsafe {
            if cell.is_null() {
                return;
            }
            let next = (*cell).next as usize;
            assert!(next < (cell as usize) || next >= (cell as usize + (*cell).size.0));
        }
    }
}

extra_only! {
    fn assert_local_cell_invariants(cell: *mut Cell) {
        assert_is_word_aligned(cell);
        assert_next_is_not_in_data(cell);
    }
}

extra_only! {
    // Assert global invariants of the given free list:
    //
    // - The free list does not have cycles
    //
    // - None of the cells within the free list are marked allocated
    //
    // - The freed cell's data is properly poisoned, i.e. there has not been any
    //   use-after-free.
    //
    // This is O(size of free list) and can be pretty slow, so try to restrict
    // its usage to verifying that a free list is still valid after mutation.
    fn assert_is_valid_free_list(head: *mut Cell, policy: &AllocPolicy) {
        unsafe {
            let mut left = head;
            assert_local_cell_invariants(left);
            if left.is_null() {
                return;
            }
            assert_is_poisoned_with_free_pattern(left, policy);

            let mut right = (*left).next;

            loop {
                assert_local_cell_invariants(right);
                if right.is_null() {
                    return;
                }
                assert_is_poisoned_with_free_pattern(right, policy);

                assert!(left != right, "free list should not have cycles");
                assert!(!Cell::is_allocated(left), "cells in free list should never be allocated");
                assert!(!Cell::is_allocated(right), "cells in free list should never be allocated");

                right = (*right).next;
                assert_local_cell_invariants(right);
                if right.is_null() {
                    return;
                }
                assert_is_poisoned_with_free_pattern(right, policy);

                left = (*left).next;
                assert_local_cell_invariants(left);
                assert_is_poisoned_with_free_pattern(left, policy);

                assert!(left != right, "free list should not have cycles");
                assert!(!Cell::is_allocated(left), "cells in free list should never be allocated");
                assert!(!Cell::is_allocated(right), "cells in free list should never be allocated");

                right = (*right).next;
            }
        }
    }
}

trait AllocPolicy {
    unsafe fn new_cell_for_free_list(&self, size: Words) -> Result<*mut Cell, ()>;

    fn min_cell_size(&self, alloc_size: Words) -> Words;

    #[cfg(feature = "extra_assertions")]
    fn allocated_sentinel(&self) -> *mut Cell;

    #[cfg(feature = "extra_assertions")]
    fn free_pattern(&self) -> u8;
}

struct LargeAllocPolicy;
static LARGE_ALLOC_POLICY: LargeAllocPolicy = LargeAllocPolicy;

impl LargeAllocPolicy {
    #[cfg(feature = "size_classes")]
    const MIN_CELL_SIZE: Words = Words(size_classes::SizeClasses::NUM_SIZE_CLASSES * 2);

    #[cfg(not(feature = "size_classes"))]
    const MIN_CELL_SIZE: Words = Words(16);
}

impl AllocPolicy for LargeAllocPolicy {
    unsafe fn new_cell_for_free_list(&self, size: Words) -> Result<*mut Cell, ()> {
        let size: Bytes = size.into();
        let pages: Pages = (size + size_of::<Cell>()).round_up_to();
        let new_pages = imp::alloc_pages(pages) as *mut Cell;
        let actual_size: Bytes = pages.into();
        Cell::write_initial(actual_size - size_of::<Cell>(), new_pages);
        Ok(new_pages)
    }

    fn min_cell_size(&self, _alloc_size: Words) -> Words {
        Self::MIN_CELL_SIZE
    }

    #[cfg(feature = "extra_assertions")]
    fn allocated_sentinel(&self) -> *mut Cell {
        Cell::LARGE_ALLOCATED_NEXT
    }

    #[cfg(feature = "extra_assertions")]
    fn free_pattern(&self) -> u8 {
        Cell::LARGE_FREE_PATTERN
    }
}

unsafe fn walk_free_list<F, T>(head: &mut *mut Cell, mut f: F) -> Result<T, ()>
where
    F: FnMut(&mut *mut Cell, *mut Cell) -> Option<T>
{
    let mut previous = head;

    loop {
        let current = *previous;
        assert_local_cell_invariants(current);

        if current.is_null() {
            return Err(());
        }

        if let Some(result) = f(previous, current) {
            return Ok(result);
        } else {
            previous = &mut (*current).next;
        }
    }
}

#[inline]
fn should_split(cell_size: Bytes, alloc_size: Words, policy: &AllocPolicy) -> bool {
    let min_cell_size: Bytes = policy.min_cell_size(alloc_size).into();
    extra_assert!(min_cell_size.0 > 0);

    let alloc_size: Bytes = alloc_size.into();
    extra_assert!(cell_size >= alloc_size);

    cell_size - alloc_size >= min_cell_size + size_of::<Cell>()
}

unsafe fn alloc_first_fit(size: Words, head: &mut *mut Cell, policy: &AllocPolicy) -> Result<*mut u8, ()> {
    extra_assert!(size.0 > 0);

    let size_in_bytes: Bytes = size.into();

    walk_free_list(head, |previous, current| {
        extra_assert_eq!(*previous, current);

        if (*current).size < size_in_bytes {
            return None;
        }

        let result = Cell::data(current);

        if should_split((*current).size, size, policy) {
            extra_assert!(size.0 <= isize::MAX as usize);

            let remainder = result.offset(size_in_bytes.0 as isize) as *mut Cell;
            Cell::write_initial((*current).size - size_in_bytes - size_of::<Cell>(), remainder);
            (*current).size = size_in_bytes;

            Cell::set_next(&mut (*remainder).next, (*current).next);
            *previous = remainder;
            assert_is_valid_free_list(*previous, policy);
        } else {
            *previous = (*current).next;
            assert_is_valid_free_list(*previous, policy);
        }

        set_allocated(current, policy);
        Some(result)
    })
}

unsafe fn alloc_with_refill(
    size: Words,
    head: &mut *mut Cell,
    policy: &AllocPolicy
) -> Result<*mut u8, ()> {
    if head.is_null() {
        Cell::insert_into_free_list(head, policy.new_cell_for_free_list(size)?, policy);
        return alloc_first_fit(size, head, policy);
    }

    if let Ok(result) = alloc_first_fit(size, head, policy) {
        return Ok(result);
    }

    Cell::insert_into_free_list(head, policy.new_cell_for_free_list(size)?, policy);
    alloc_first_fit(size, head, policy)
}

/// A wee allocator.
///
/// # Safety
///
/// When used in unix environments, cannot move in memory. Typically not an
/// issue if you're just using this as a `static` global allocator.
pub struct WeeAlloc {
    head: imp::Exclusive<*mut Cell>,

    #[cfg(feature = "size_classes")]
    size_classes: size_classes::SizeClasses,
}

unsafe impl Sync for WeeAlloc {}

impl ConstInit for WeeAlloc {
    const INIT: WeeAlloc = WeeAlloc {
        head: imp::Exclusive::INIT,

        #[cfg(feature = "size_classes")]
        size_classes: size_classes::SizeClasses::INIT,
    };
}

impl WeeAlloc {
    /// TODO FITZGEN
    pub const INIT: Self = <Self as ConstInit>::INIT;

    #[cfg(feature = "size_classes")]
    #[inline]
    unsafe fn with_free_list_and_policy_for_size<F, T>(
        &self,
        size: Words,
        f: F
    ) -> T
    where
        F: for<'a> FnOnce(&'a mut *mut Cell, &'a AllocPolicy) -> T
    {
        extra_assert!(size.0 > 0);
        if let Some(head) = self.size_classes.get(size) {
            let policy = size_classes::SizeClassAllocPolicy(&self.head);
            let policy = &policy as &AllocPolicy;
            head.with_exclusive_access(|head| {
                f(head, policy)
            })
        } else {
            let policy = &LARGE_ALLOC_POLICY as &AllocPolicy;
            self.head.with_exclusive_access(|head| {
                f(head, policy)
            })
        }
    }

    #[cfg(not(feature = "size_classes"))]
    #[inline]
    unsafe fn with_free_list_and_policy_for_size<F, T>(
        &self,
        size: Words,
        f: F
    ) -> T
    where
        F: for<'a> FnOnce(&'a mut *mut Cell, &'a AllocPolicy) -> T
    {
        extra_assert!(size.0 > 0);
        let policy = &LARGE_ALLOC_POLICY as &AllocPolicy;
        self.head.with_exclusive_access(|head| {
            f(head, policy)
        })
    }

}

unsafe impl<'a> Alloc for &'a WeeAlloc {
    unsafe fn alloc(&mut self, layout: Layout) -> Result<*mut u8, AllocErr> {
        if layout.align() > ::core::mem::size_of::<usize>() {
            return Err(AllocErr::Unsupported {
                details: "wee_alloc cannot align to more than word alignment"
            });
        }

        let size = Bytes(layout.size());
        if size.0 == 0 {
            return Ok(0x1 as *mut u8);
        }

        let size: Words = size.round_up_to();
        self.with_free_list_and_policy_for_size(size, |head, policy| {
            alloc_with_refill(size, head, policy).map_err(move |()| AllocErr::Exhausted {
                request: layout
            })
        })
    }

    unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        let size = Bytes(layout.size());

        if size.0 == 0 || ptr.is_null() {
            return;
        }

        let size: Words = size.round_up_to();
        self.with_free_list_and_policy_for_size(size, |head, policy| {
            let cell = (ptr as *mut Cell).offset(-1);
            extra_assert!((*cell).size >= size.into());
            assert_is_allocated(cell, policy);

            Cell::insert_into_free_list(head, cell, policy);
        });
    }
}

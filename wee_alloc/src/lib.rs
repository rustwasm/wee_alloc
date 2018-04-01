/*!

[![](https://docs.rs/wee_alloc/badge.svg)](https://docs.rs/wee_alloc/)
[![](https://img.shields.io/crates/v/wee_alloc.svg)](https://crates.io/crates/wee_alloc)
[![](https://img.shields.io/crates/d/wee_alloc.svg)](https://crates.io/crates/wee_alloc)
[![Travis CI Build Status](https://travis-ci.org/fitzgen/wee_alloc.svg?branch=master)](https://travis-ci.org/fitzgen/wee_alloc)
[![AppVeyor Build status](https://ci.appveyor.com/api/projects/status/bqh8elm9wy0k5x2r/branch/master?svg=true)](https://ci.appveyor.com/project/fitzgen/wee-alloc/branch/master)

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
  code size by about 450 bytes (up to a total of ~1.2K).

- **extra_assertions**: Enable various extra, expensive integrity assertions and
  defensive mechanisms, such as poisoning freed memory. This incurs a large
  runtime overhead. It is useful when debugging a use-after-free or `wee_alloc`
  itself.

## Implementation Notes and Constraints

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

#![deny(missing_docs)]
#![cfg_attr(not(feature = "use_std_for_test_debugging"), no_std)]
#![feature(alloc, allocator_api, core_intrinsics, global_allocator)]
#![cfg_attr(target_arch = "wasm32", feature(link_llvm_intrinsics))]

#[macro_use]
extern crate cfg_if;

extern crate alloc;

#[cfg(feature = "use_std_for_test_debugging")]
extern crate core;

extern crate memory_units;

#[macro_use]
mod extra_assert;

cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        mod imp_wasm32;
        use imp_wasm32 as imp;
    } else if #[cfg(unix)] {
        extern crate libc;
        extern crate mmap_alloc;
        mod imp_unix;
        use imp_unix as imp;
    } else if #[cfg(windows)] {
        extern crate winapi;
        mod imp_windows;
        use imp_windows as imp;
    } else {
        compile_error! {
            "There is no `wee_alloc` implementation for this target; want to send a pull request? :)"
        }
    }
}

mod const_init;

#[cfg(feature = "size_classes")]
mod size_classes;

use alloc::heap::{Alloc, AllocErr, Layout};
use const_init::ConstInit;
use core::cmp;
use core::isize;
use core::marker::Sync;
use core::mem;
use core::ptr;
use memory_units::{size_of, Bytes, Pages, RoundUpTo, Words};

/// The WebAssembly page size, in bytes.
pub const PAGE_SIZE: Bytes = Bytes(65536);

extra_only! {
    fn assert_is_word_aligned<T>(ptr: *mut T) {
        assert_aligned_to(ptr, size_of::<usize>());
    }
}

extra_only! {
    fn assert_aligned_to<T>(ptr: *mut T, align: Bytes) {
        extra_assert_eq!(
            ptr as usize % align.0,
            0,
            "{:p} is not aligned to {}",
            ptr,
            align.0
        );
    }
}

#[repr(C)]
struct CellHeader {
    next_cell_raw: ptr::NonNull<CellHeader>,
    prev_cell_raw: *mut CellHeader,
}

#[repr(C)]
struct AllocatedCell {
    header: CellHeader,
}

#[test]
fn allocated_cell_layout() {
    assert_eq!(
        size_of::<CellHeader>(),
        size_of::<AllocatedCell>(),
        "Safety and correctness depends on AllocatedCell being the same as CellHeader"
    );

    assert_eq!(
        mem::align_of::<CellHeader>(),
        mem::align_of::<AllocatedCell>()
    );
}

#[repr(C)]
struct FreeCell {
    header: CellHeader,
    next_free_raw: *mut FreeCell,
}

#[test]
fn free_cell_layout() {
    assert_eq!(
        size_of::<CellHeader>() + Bytes(1),
        size_of::<FreeCell>(),
        "Safety and correctness depends on FreeCell being only one word larger than CellHeader"
    );

    assert_eq!(
        mem::align_of::<CellHeader>(),
        mem::align_of::<AllocatedCell>()
    );
}

#[cfg(feature = "extra_assertions")]
impl CellHeader {
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
    // use-after-free, or (b) there is a bug in `wee_alloc` and its main free
    // list implementation.
    const LARGE_FREE_PATTERN: u8 = 0x57;
}

impl CellHeader {
    // Low bits in `CellHeader::next_cell_raw`.
    //
    // If `CellHeader::IS_ALLOCATED` is set, then the cell is allocated, and
    // should never be in the free list. If the bit is not set, then this cell
    // is free, and must be in the free list (or is in the process of being
    // added to the free list).
    //
    // The `CellHeader::next_cell_raw` pointer (after appropriate masking)
    // always points to the byte just *after* this cell. If the
    // `CellHeader::NEXT_CELL_IS_INVALID` bit is not set, then it points to the
    // next cell. If that bit is set, then it points to the invalid memory that
    // follows this cell.
    const IS_ALLOCATED: usize = 0b01;
    const NEXT_CELL_IS_INVALID: usize = 0b10;
    const MASK: usize = !0b11;

    #[test]
    fn can_use_low_bits() {
        assert!(
            mem::align_of::<*mut u8>() >= 0b100,
            "we rely on being able to stick tags into the lowest two bits"
        );
    }

    fn is_allocated(&self) -> bool {
        self.next_cell_raw.as_ptr() as usize & Self::IS_ALLOCATED != 0
    }

    fn is_free(&self) -> bool {
        !self.is_allocated()
    }

    fn set_allocated(&mut self) {
        let next = self.next_cell_raw.as_ptr() as usize;
        let next = next | Self::IS_ALLOCATED;
        extra_assert!(next != 0);
        self.next_cell_raw = unsafe { ptr::NonNull::new_unchecked(next as *mut CellHeader) };
    }

    fn set_free(&mut self) {
        let next = self.next_cell_raw.as_ptr() as usize;
        let next = next & !Self::IS_ALLOCATED;
        extra_assert!(next != 0);
        self.next_cell_raw = unsafe { ptr::NonNull::new_unchecked(next as *mut CellHeader) };
    }

    fn next_cell_is_invalid(&self) -> bool {
        self.next_cell_raw.as_ptr() as usize & Self::NEXT_CELL_IS_INVALID != 0
    }

    fn next_cell_unchecked(&self) -> *mut CellHeader {
        let ptr = self.next_cell_raw.as_ptr() as usize;
        let ptr = ptr & Self::MASK;
        let ptr = ptr as *mut CellHeader;
        extra_assert!(!ptr.is_null());
        assert_is_word_aligned(ptr);
        ptr
    }

    fn next_cell(&self) -> Option<*mut CellHeader> {
        if self.next_cell_is_invalid() {
            None
        } else {
            Some(self.next_cell_unchecked())
        }
    }

    fn prev_cell(&self) -> Option<*mut CellHeader> {
        if self.prev_cell_raw.is_null() {
            None
        } else {
            Some(self.prev_cell_raw)
        }
    }

    fn size(&self) -> Bytes {
        let data = unsafe { (self as *const CellHeader as *mut CellHeader).offset(1) };
        assert_is_word_aligned(data);
        let data = data as usize;

        let next = self.next_cell_unchecked();
        assert_is_word_aligned(next);
        let next = next as usize;

        extra_assert!(next > data);
        Bytes(next - data)
    }

    #[cfg(feature = "extra_assertions")]
    fn as_free_cell(&self) -> Option<&FreeCell> {
        if self.is_free() {
            Some(unsafe { mem::transmute(self) })
        } else {
            None
        }
    }

    fn as_free_cell_mut(&mut self) -> Option<&mut FreeCell> {
        if self.is_free() {
            Some(unsafe { mem::transmute(self) })
        } else {
            None
        }
    }

    // Get a pointer to this cell's data without regard to whether this cell is
    // allocated or free.
    unsafe fn unchecked_data(&self) -> *mut u8 {
        (self as *const CellHeader).offset(1) as *const u8 as *mut u8
    }

    // Is this cell aligned to the given power-of-2 alignment?
    fn is_aligned_to<B: Into<Bytes>>(&self, align: B) -> bool {
        let align = align.into();
        extra_assert!(align.0.is_power_of_two());

        let data = unsafe { self.unchecked_data() } as usize;
        data & (align.0 - 1) == 0
    }
}

impl FreeCell {
    // Low bits in `FreeCell::next_free_raw`.
    //
    // If `NEXT_FREE_CELL_CAN_MERGE` is set, then the following invariants hold
    // true:
    //
    // * `FreeCell::next_free_raw` (and'd with the mask) is not null.
    // * `FreeCell::next_free_raw` is the adjacent `CellHeader::prev_cell_raw`.
    //
    // Therefore, this free cell can be merged into a single, larger, contiguous
    // free cell with its previous neighbor, which is also the next cell in the
    // free list.
    const NEXT_FREE_CELL_CAN_MERGE: usize = 0b01;
    const _RESERVED: usize = 0b10;
    const MASK: usize = !0b11;

    fn next_free_can_merge(&self) -> bool {
        self.next_free_raw as usize & Self::NEXT_FREE_CELL_CAN_MERGE != 0
    }

    fn set_next_free_can_merge(&mut self) {
        let next_free = self.next_free_raw as usize;
        let next_free = next_free | Self::NEXT_FREE_CELL_CAN_MERGE;
        self.next_free_raw = next_free as *mut FreeCell;
    }

    fn next_free(&self) -> *mut FreeCell {
        let next_free = self.next_free_raw as usize & Self::MASK;
        next_free as *mut FreeCell
    }

    unsafe fn from_uninitialized<'a>(
        raw: *mut u8,
        next_cell: ptr::NonNull<CellHeader>,
        prev_cell: Option<*mut CellHeader>,
        next_free: Option<*mut FreeCell>,
        policy: &AllocPolicy,
    ) -> *mut FreeCell {
        extra_assert!(!raw.is_null());
        assert_is_word_aligned(raw);
        extra_assert!((raw as usize) < (next_cell.as_ptr() as usize));
        extra_assert!((next_cell.as_ptr() as usize) - (raw as usize) >= size_of::<usize>().0);

        let prev_cell = prev_cell.unwrap_or(ptr::null_mut());
        let next_free = next_free.unwrap_or(ptr::null_mut());

        let raw = raw as *mut FreeCell;
        ptr::write(
            raw,
            FreeCell {
                header: CellHeader {
                    next_cell_raw: next_cell,
                    prev_cell_raw: prev_cell,
                },
                next_free_raw: next_free,
            },
        );
        write_free_pattern(&mut *raw, policy);
        raw
    }

    fn into_allocated_cell(&mut self, policy: &AllocPolicy) -> &mut AllocatedCell {
        assert_local_cell_invariants(&mut self.header);
        assert_is_poisoned_with_free_pattern(self, policy);

        self.header.set_allocated();
        unsafe { mem::transmute(self) }
    }

    // Try and satisfy the given allocation request with this cell.
    fn try_alloc<'a>(
        &'a mut self,
        previous: &mut *mut FreeCell,
        alloc_size: Words,
        align: Bytes,
        policy: &AllocPolicy,
    ) -> Option<&'a mut AllocatedCell> {
        extra_assert!(alloc_size.0 > 0);
        extra_assert!(align.0 > 0);
        extra_assert!(align.0.is_power_of_two());

        // First, do a quick check that this cell can hold an allocation of the
        // requested size.
        let size: Bytes = alloc_size.into();
        if self.header.size() < size {
            return None;
        }

        // Next, try and allocate by splitting this cell in two, and returning
        // the second half.
        //
        // We allocate from the end, rather than the beginning, of this cell for
        // two reasons:
        //
        // 1. It allows us to satisfy alignment requests, since we can choose to
        //    split at some alignment and return the aligned cell at the end.
        //
        // 2. It involves fewer writes to maintain the free list links, since this
        //    cell is already in the free list, and after splitting that means that
        //    the front half is also already in the free list.
        let next = self.header.next_cell_unchecked() as usize;
        let align: Bytes = align.into();
        let split_and_aligned = (next - size.0) & !(align.0 - 1);
        let data = unsafe { self.header.unchecked_data() } as usize;
        let min_cell_size: Bytes = policy.min_cell_size(alloc_size).into();
        if data + size_of::<CellHeader>().0 + min_cell_size.0 <= split_and_aligned {
            let split_cell_head = split_and_aligned - size_of::<CellHeader>().0;
            let split_cell = unsafe {
                &mut *FreeCell::from_uninitialized(
                    split_cell_head as *mut u8,
                    self.header.next_cell_raw,
                    Some(&mut self.header),
                    None,
                    policy,
                )
            };

            if let Some(next) = self.header.next_cell() {
                unsafe {
                    (*next).prev_cell_raw = &mut split_cell.header;
                }
            }

            self.header.next_cell_raw =
                unsafe { ptr::NonNull::new_unchecked(&mut split_cell.header) };

            return Some(split_cell.into_allocated_cell(policy));
        }

        // There isn't enough room to split this cell and still satisfy the
        // requested allocation. Because of the early check, we know this cell
        // is large enough to fit the requested size, but is the cell's data
        // properly aligned?
        let align: Bytes = align.into();
        if self.header.is_aligned_to(align) {
            *previous = self.next_free();
            let allocated = self.into_allocated_cell(policy);
            assert_is_valid_free_list(*previous, policy);
            return Some(allocated);
        }

        None
    }

    fn insert_into_free_list<'a>(
        &'a mut self,
        head: &'a mut *mut FreeCell,
        policy: &AllocPolicy,
    ) -> &'a mut *mut FreeCell {
        extra_assert!(!self.next_free_can_merge());
        extra_assert!(self.next_free().is_null());
        self.next_free_raw = *head;
        *head = self;
        assert_is_valid_free_list(*head, policy);
        head
    }

    #[cfg(feature = "extra_assertions")]
    fn tail_data(&mut self) -> *mut u8 {
        let data = unsafe { (self as *mut FreeCell).offset(1) as *mut u8 };
        assert_is_word_aligned(data);
        data
    }

    #[cfg(feature = "extra_assertions")]
    fn tail_data_size(&self) -> Bytes {
        let size = self.header.size();
        extra_assert!(size >= size_of::<usize>());
        // Subtract a word from the size, since `FreeCell::next_free` uses it.
        size - size_of::<usize>()
    }
}

impl AllocatedCell {
    unsafe fn into_free_cell(&mut self, policy: &AllocPolicy) -> &mut FreeCell {
        assert_local_cell_invariants(&mut self.header);

        self.header.set_free();
        let free: &mut FreeCell = mem::transmute(self);
        write_free_pattern(free, policy);
        free.next_free_raw = ptr::null_mut();
        free
    }

    fn data(&self) -> *mut u8 {
        let cell = &self.header as *const CellHeader as *mut CellHeader;
        extra_assert!(!cell.is_null());
        assert_local_cell_invariants(cell);
        unsafe { cell.offset(1) as *mut u8 }
    }
}

extra_only! {
    fn write_free_pattern(cell: &mut FreeCell, policy: &AllocPolicy) {
        unsafe {
            let data = cell.tail_data();
            let size: Bytes = cell.tail_data_size();
            let pattern = policy.free_pattern();
            ptr::write_bytes(data, pattern, size.0);
        }
    }
}

extra_only! {
    fn assert_is_poisoned_with_free_pattern(cell: &mut FreeCell, policy: &AllocPolicy) {
        use core::slice;
        unsafe {
            let size: Bytes = cell.tail_data_size();
            let data = cell.tail_data();
            let data = slice::from_raw_parts(data, size.0);
            let pattern = policy.free_pattern();
            extra_assert!(data.iter().all(|byte| *byte == pattern));
        }
    }
}

extra_only! {
    fn assert_local_cell_invariants(cell: *mut CellHeader) {
        assert_is_word_aligned(cell);
        unsafe {
            if let Some(cell_ref) = cell.as_ref() {
                assert!(cell_ref.size() >= size_of::<usize>());

                if let Some(prev) = cell_ref.prev_cell().and_then(|p| p.as_ref()) {
                    assert!(prev.size() >= size_of::<usize>());
                    assert!(!prev.next_cell_is_invalid());
                    assert_eq!(prev.next_cell_unchecked(), cell, "next(prev(cell)) == cell");
                }

                if let Some(next) = cell_ref.next_cell() {
                    assert!(!next.is_null());
                    let next = &*next;
                    assert!(next.size() >= size_of::<usize>());
                    assert_eq!(next.prev_cell_raw, cell, "prev(next(cell)) == cell");
                }

                if let Some(free) = cell_ref.as_free_cell() {
                    if free.next_free_can_merge() {
                        let prev_cell = free.header.prev_cell().expect(
                            "if the next free cell (aka prev_cell) can merge, \
                             prev_cell had better exist"
                        );
                        assert!(!prev_cell.is_null());
                        assert!(
                            (*prev_cell).is_free(),
                            "prev_cell is free, when NEXT_FREE_CELL_CAN_MERGE bit is set"
                        );
                        assert_eq!(
                            free.next_free() as *mut CellHeader,
                            prev_cell,
                            "next_free == prev_cell, when NEXT_FREE_CAN_MERGE bit is set"
                        );
                    }
                }
            }
        }
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
    fn assert_is_valid_free_list(head: *mut FreeCell, policy: &AllocPolicy) {
        unsafe {
            let mut left = head;
            assert_local_cell_invariants(left as *mut CellHeader);
            if left.is_null() {
                return;
            }
            assert_is_poisoned_with_free_pattern(&mut *left, policy);

            let mut right = (*left).next_free();

            loop {
                assert_local_cell_invariants(right as *mut CellHeader);
                if right.is_null() {
                    return;
                }
                assert_is_poisoned_with_free_pattern(&mut *right, policy);

                assert!(left != right, "free list should not have cycles");
                assert!((*right).header.is_free(), "cells in free list should never be allocated");
                assert!((*left).header.is_free(), "cells in free list should never be allocated");

                right = (*right).next_free();
                assert_local_cell_invariants(right as *mut CellHeader);
                if right.is_null() {
                    return;
                }
                assert_is_poisoned_with_free_pattern(&mut *right, policy);

                left = (*left).next_free();
                assert_local_cell_invariants(left as *mut CellHeader);
                assert_is_poisoned_with_free_pattern(&mut *left, policy);

                assert!(left != right, "free list should not have cycles");
                assert!((*right).header.is_free(), "cells in free list should never be allocated");
                assert!((*left).header.is_free(), "cells in free list should never be allocated");

                right = (*right).next_free();
            }
        }
    }
}

trait AllocPolicy {
    unsafe fn new_cell_for_free_list(&self, size: Words, align: Bytes)
        -> Result<*mut FreeCell, ()>;

    fn min_cell_size(&self, alloc_size: Words) -> Words;

    fn should_merge_adjacent_free_cells(&self) -> bool;

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
    unsafe fn new_cell_for_free_list(
        &self,
        size: Words,
        align: Bytes,
    ) -> Result<*mut FreeCell, ()> {
        // To assure that an allocation will always succeed after refilling the
        // free list with this new cell, make sure that we allocate enough to
        // fulfill the requested alignment, and still have the minimum cell size
        // left over.
        let size: Bytes = cmp::max(
            size.into(),
            ((align + Self::MIN_CELL_SIZE) * Words(2)).into(),
        );
        let pages: Pages = (size + size_of::<CellHeader>()).round_up_to();
        let new_pages = imp::alloc_pages(pages)?;
        let allocated_size: Bytes = pages.into();
        let next_cell = new_pages.offset(allocated_size.0 as isize);
        let next_cell = next_cell as usize | CellHeader::NEXT_CELL_IS_INVALID;
        extra_assert!(next_cell != 0);
        let next_cell = ptr::NonNull::new_unchecked(next_cell as *mut CellHeader);
        Ok(FreeCell::from_uninitialized(
            new_pages,
            next_cell,
            None,
            None,
            self as &AllocPolicy,
        ))
    }

    fn min_cell_size(&self, _alloc_size: Words) -> Words {
        Self::MIN_CELL_SIZE
    }

    fn should_merge_adjacent_free_cells(&self) -> bool {
        true
    }

    #[cfg(feature = "extra_assertions")]
    fn free_pattern(&self) -> u8 {
        CellHeader::LARGE_FREE_PATTERN
    }
}

unsafe fn walk_free_list<F, T>(
    head: &mut *mut FreeCell,
    policy: &AllocPolicy,
    mut f: F,
) -> Result<T, ()>
where
    F: FnMut(&mut *mut FreeCell, &mut FreeCell) -> Option<T>,
{
    // The previous cell in the free list (not to be confused with the current
    // cell's previously _adjacent_ cell).
    let mut previous_free = head;

    loop {
        let current_free = *previous_free;
        assert_local_cell_invariants(current_free as *mut CellHeader);

        if current_free.is_null() {
            return Err(());
        }

        let mut current_free = &mut *current_free;

        // Now check if this cell can merge with the next cell in the free
        // list.
        //
        // We don't re-check `policy.should_merge_adjacent_free_cells()` because
        // the `NEXT_FREE_CELL_CAN_MERGE` bit only gets set after checking with
        // the policy.
        while current_free.next_free_can_merge() {
            extra_assert!(policy.should_merge_adjacent_free_cells());

            let prev_adjacent = current_free.header.prev_cell_raw as *mut FreeCell;
            extra_assert_eq!(prev_adjacent, current_free.next_free());
            let prev_adjacent = &mut *prev_adjacent;

            (*prev_adjacent).header.next_cell_raw = current_free.header.next_cell_raw;
            if let Some(next) = current_free.header.next_cell() {
                (*next).prev_cell_raw = &mut prev_adjacent.header;
            }

            *previous_free = prev_adjacent;
            current_free = prev_adjacent;

            write_free_pattern(current_free, policy);
            assert_local_cell_invariants(&mut current_free.header);
        }

        if let Some(result) = f(previous_free, current_free) {
            return Ok(result);
        }

        previous_free = &mut current_free.next_free_raw;
    }
}

/// Do a first-fit allocation from the given free list.
unsafe fn alloc_first_fit(
    size: Words,
    align: Bytes,
    head: &mut *mut FreeCell,
    policy: &AllocPolicy,
) -> Result<*mut u8, ()> {
    extra_assert!(size.0 > 0);

    walk_free_list(head, policy, |previous, current| {
        extra_assert_eq!(*previous, current as *mut _);

        if let Some(allocated) = current.try_alloc(previous, size, align, policy) {
            assert_aligned_to(allocated.data(), align);
            return Some(allocated.data());
        }

        None
    })
}

unsafe fn alloc_with_refill(
    size: Words,
    align: Bytes,
    head: &mut *mut FreeCell,
    policy: &AllocPolicy,
) -> Result<*mut u8, ()> {
    if let Ok(result) = alloc_first_fit(size, align, head, policy) {
        return Ok(result);
    }

    let cell = policy.new_cell_for_free_list(size, align)?;
    let head = (*cell).insert_into_free_list(head, policy);

    let result = alloc_first_fit(size, align, head, policy);
    extra_assert!(
        result.is_ok(),
        "if refilling the free list succeeds, then retrying the allocation \
         should also always succeed"
    );
    result
}

/// A wee allocator.
///
/// # Safety
///
/// When used in unix environments, cannot move in memory. Typically not an
/// issue if you're just using this as a `static` global allocator.
pub struct WeeAlloc {
    head: imp::Exclusive<*mut FreeCell>,

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
    /// An initial `const` default construction of a `WeeAlloc` allocator.
    ///
    /// This is usable for initializing `static`s that get set as the global
    /// allocator.
    pub const INIT: Self = <Self as ConstInit>::INIT;

    #[cfg(feature = "size_classes")]
    unsafe fn with_free_list_and_policy_for_size<F, T>(&self, size: Words, align: Bytes, f: F) -> T
    where
        F: for<'a> FnOnce(&'a mut *mut FreeCell, &'a AllocPolicy) -> T,
    {
        extra_assert!(size.0 > 0);
        extra_assert!(align.0 > 0);

        let align: Bytes = align.into();
        if align <= size_of::<usize>() {
            if let Some(head) = self.size_classes.get(size) {
                let policy = size_classes::SizeClassAllocPolicy(&self.head);
                let policy = &policy as &AllocPolicy;
                return head.with_exclusive_access(|head| f(head, policy));
            }
        }

        let policy = &LARGE_ALLOC_POLICY as &AllocPolicy;
        self.head.with_exclusive_access(|head| f(head, policy))
    }

    #[cfg(not(feature = "size_classes"))]
    unsafe fn with_free_list_and_policy_for_size<F, T>(&self, size: Words, _align: Bytes, f: F) -> T
    where
        F: for<'a> FnOnce(&'a mut *mut FreeCell, &'a AllocPolicy) -> T,
    {
        extra_assert!(size.0 > 0);
        let policy = &LARGE_ALLOC_POLICY as &AllocPolicy;
        self.head.with_exclusive_access(|head| f(head, policy))
    }
}

unsafe impl<'a> Alloc for &'a WeeAlloc {
    unsafe fn alloc(&mut self, layout: Layout) -> Result<*mut u8, AllocErr> {
        let size = Bytes(layout.size());
        let align = if layout.align() == 0 {
            Bytes(1)
        } else {
            Bytes(layout.align())
        };

        if size.0 == 0 {
            // Ensure that our made up pointer is properly aligned by using the
            // alignment as the pointer.
            return Ok(align.0 as *mut u8);
        }

        let size: Words = size.round_up_to();

        self.with_free_list_and_policy_for_size(size, align, |head, policy| {
            assert_is_valid_free_list(*head, policy);
            alloc_with_refill(size, align, head, policy)
                .map_err(|()| AllocErr::Exhausted { request: layout })
        })
    }

    unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        let size = Bytes(layout.size());
        if size.0 == 0 || ptr.is_null() {
            return;
        }

        let size: Words = size.round_up_to();
        let align = Bytes(layout.align());

        self.with_free_list_and_policy_for_size(size, align, |head, policy| {
            let cell = (ptr as *mut CellHeader).offset(-1);
            let cell = &mut *cell;

            extra_assert!(cell.size() >= size.into());
            extra_assert!(cell.is_allocated());
            let cell: &mut AllocatedCell = mem::transmute(cell);

            let free = cell.into_free_cell(policy);

            if policy.should_merge_adjacent_free_cells() {
                // Merging with the _previous_ adjacent cell is easy: it is
                // already in the free list, so folding this cell into it is all
                // that needs to be done. The free list can be left alone.
                //
                // Merging with the _next_ adjacent cell is a little harder. It
                // is already in the free list, but we need to splice it out
                // from the free list, since its header will become invalid
                // after consolidation, and it is *this* cell's header that
                // needs to be in the free list. But we don't have access to the
                // pointer pointing to the soon-to-be-invalid header, and
                // therefore can't adjust that pointer. So we have a delayed
                // consolidation scheme. We insert this cell just after the next
                // adjacent cell in the free list, and set the next adjacent
                // cell's `NEXT_FREE_CAN_MERGE` bit. The next time that we walk
                // the free list for allocation, the bit will be checked and the
                // consolidation will happen at that time.
                //
                // If _both_ the previous and next adjacent cells are free, we
                // are faced with a dilemma. We cannot merge all previous,
                // current, and next cells together because our singly-linked
                // free list doesn't allow for that kind of arbitrary appending
                // and splicing. There are a few different kinds of tricks we
                // could pull here, but they would increase implementation
                // complexity and code size. Instead, we use a heuristic to
                // choose whether to merge with the previous or next adjacent
                // cell. We could choose to merge with whichever neighbor cell
                // is smaller or larger, but we don't. We prefer the previous
                // adjacent cell because we can greedily consolidate with it
                // immediately, whereas the consolidating with the next adjacent
                // cell must be delayed, as explained above.

                if let Some(prev) = free.header
                    .prev_cell()
                    .and_then(|p| (*p).as_free_cell_mut())
                {
                    prev.header.next_cell_raw = free.header.next_cell_raw;
                    if let Some(next) = free.header.next_cell() {
                        (*next).prev_cell_raw = &mut prev.header;
                    }

                    write_free_pattern(prev, policy);
                    assert_is_valid_free_list(*head, policy);
                    return;
                }

                if let Some(next) = free.header
                    .next_cell()
                    .and_then(|n| (*n).as_free_cell_mut())
                {
                    free.next_free_raw = next.next_free();
                    next.next_free_raw = free;
                    next.set_next_free_can_merge();

                    assert_is_valid_free_list(*head, policy);
                    return;
                }
            }

            // Either we don't want to merge cells for the current policy, or we
            // didn't have the opportunity to do any merging with our adjacent
            // neighbors. In either case, push this cell onto the front of the
            // free list.
            let _head = free.insert_into_free_list(head, policy);
        });
    }
}

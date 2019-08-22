/*!

## About

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

## Using `wee_alloc` as the Global Allocator

```
extern crate wee_alloc;

// Use `wee_alloc` as the global allocator.
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;
# fn main() {}
```

## `cargo` Features

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
[CONTRIBUTING.md](https://github.com/rustwasm/wee_alloc/blob/master/CONTRIBUTING.md)
for hacking!

 */

#![deny(missing_docs)]
#![cfg_attr(not(feature = "use_std_for_test_debugging"), no_std)]
#![cfg_attr(feature = "nightly", feature(allocator_api, core_intrinsics))]

#[macro_use]
extern crate cfg_if;

#[cfg(feature = "nightly")]
extern crate alloc;

#[cfg(feature = "use_std_for_test_debugging")]
extern crate core;
#[cfg(feature = "static_array_backend")]
extern crate spin;

extern crate memory_units;

#[macro_use]
mod extra_assert;

cfg_if! {
    if #[cfg(feature = "static_array_backend")] {
        mod imp_static_array;
        use imp_static_array as imp;
    } else if #[cfg(target_arch = "wasm32")] {
        mod imp_wasm32;
        use imp_wasm32 as imp;
    } else if #[cfg(unix)] {
        extern crate libc;
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
mod neighbors;
#[cfg(feature = "size_classes")]
mod size_classes;

cfg_if! {
    if #[cfg(feature = "nightly")] {
        use core::alloc::{Alloc, AllocErr};
    } else {
        pub(crate) struct AllocErr;
    }
}

use const_init::ConstInit;
use core::alloc::{GlobalAlloc, Layout};
use core::cell::Cell;
use core::cmp;
use core::marker::Sync;
use core::mem;
use core::ptr::{self, NonNull};
use memory_units::{size_of, Bytes, Pages, RoundUpTo, Words};
use neighbors::Neighbors;

/// The WebAssembly page size, in bytes.
pub const PAGE_SIZE: Bytes = Bytes(65536);

extra_only! {
    fn assert_is_word_aligned<T>(ptr: *const T) {
        assert_aligned_to(ptr, size_of::<usize>());
    }
}

extra_only! {
    fn assert_aligned_to<T>(ptr: *const T, align: Bytes) {
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
#[derive(Default, Debug)]
struct CellHeader<'a> {
    neighbors: Neighbors<'a, CellHeader<'a>>,
}

impl<'a> AsRef<Neighbors<'a, CellHeader<'a>>> for CellHeader<'a> {
    fn as_ref(&self) -> &Neighbors<'a, CellHeader<'a>> {
        &self.neighbors
    }
}

unsafe impl<'a> neighbors::HasNeighbors<'a, CellHeader<'a>> for CellHeader<'a> {
    #[inline]
    unsafe fn next_checked(
        neighbors: &Neighbors<'a, CellHeader<'a>>,
        next: *const CellHeader<'a>,
    ) -> Option<&'a CellHeader<'a>> {
        if next.is_null() || CellHeader::next_cell_is_invalid(neighbors) {
            None
        } else {
            Some(&*next)
        }
    }

    #[inline]
    unsafe fn prev_checked(
        _neighbors: &Neighbors<'a, CellHeader<'a>>,
        prev: *const CellHeader<'a>,
    ) -> Option<&'a CellHeader<'a>> {
        if prev.is_null() {
            None
        } else {
            Some(&*prev)
        }
    }
}

#[repr(C)]
#[derive(Debug)]
struct AllocatedCell<'a> {
    header: CellHeader<'a>,
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
#[derive(Debug)]
struct FreeCell<'a> {
    header: CellHeader<'a>,
    next_free_raw: Cell<*const FreeCell<'a>>,
}

#[test]
fn free_cell_layout() {
    assert_eq!(
        size_of::<CellHeader>() + Words(1),
        size_of::<FreeCell>(),
        "Safety and correctness depends on FreeCell being only one word larger than CellHeader"
    );

    assert_eq!(
        mem::align_of::<CellHeader>(),
        mem::align_of::<AllocatedCell>()
    );
}

#[cfg(feature = "extra_assertions")]
impl<'a> CellHeader<'a> {
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

impl<'a> CellHeader<'a> {
    // ### Semantics of Low Bits in Neighbors Pointers
    //
    // If `self.neighbors.next_bit_1` is set, then the cell is allocated, and
    // should never be in the free list. If the bit is not set, then this cell
    // is free, and must be in the free list (or is in the process of being
    // added to the free list).
    //
    // The `self.neighbors.next` pointer always points to the byte just *after*
    // this cell. If the `self.neighbors.next_bit_2` bit is not set, then it
    // points to the next cell. If that bit is set, then it points to the
    // invalid memory that follows this cell.

    fn is_allocated(&self) -> bool {
        self.neighbors.get_next_bit_1()
    }

    fn is_free(&self) -> bool {
        !self.is_allocated()
    }

    fn set_allocated(neighbors: &Neighbors<'a, Self>) {
        neighbors.set_next_bit_1();
    }

    fn set_free(neighbors: &Neighbors<'a, Self>) {
        neighbors.clear_next_bit_1();
    }

    fn next_cell_is_invalid(neighbors: &Neighbors<'a, Self>) -> bool {
        neighbors.get_next_bit_2()
    }

    fn set_next_cell_is_invalid(neighbors: &Neighbors<'a, Self>) {
        neighbors.set_next_bit_2();
    }

    fn clear_next_cell_is_invalid(neighbors: &Neighbors<'a, Self>) {
        neighbors.clear_next_bit_2();
    }

    fn size(&self) -> Bytes {
        let data = unsafe { (self as *const CellHeader<'a>).offset(1) };
        assert_is_word_aligned(data);
        let data = data as usize;

        let next = self.neighbors.next_unchecked();
        assert_is_word_aligned(next);
        let next = next as usize;

        extra_assert!(
            next > data,
            "the next cell ({:p}) should always be after the data ({:p})",
            next as *const (),
            data as *const ()
        );
        Bytes(next - data)
    }

    fn as_free_cell(&self) -> Option<&FreeCell<'a>> {
        if self.is_free() {
            Some(unsafe { mem::transmute(self) })
        } else {
            None
        }
    }

    // Get a pointer to this cell's data without regard to whether this cell is
    // allocated or free.
    unsafe fn unchecked_data(&self) -> *const u8 {
        (self as *const CellHeader).offset(1) as *const u8
    }

    // Is this cell aligned to the given power-of-2 alignment?
    fn is_aligned_to<B: Into<Bytes>>(&self, align: B) -> bool {
        let align = align.into();
        extra_assert!(align.0.is_power_of_two());

        let data = unsafe { self.unchecked_data() } as usize;
        data & (align.0 - 1) == 0
    }
}

impl<'a> FreeCell<'a> {
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
        self.next_free_raw.get() as usize & Self::NEXT_FREE_CELL_CAN_MERGE != 0
    }

    fn set_next_free_can_merge(&self) {
        let next_free = self.next_free_raw.get() as usize;
        let next_free = next_free | Self::NEXT_FREE_CELL_CAN_MERGE;
        self.next_free_raw.set(next_free as *const FreeCell);
    }

    fn clear_next_free_can_merge(&self) {
        let next_free = self.next_free_raw.get() as usize;
        let next_free = next_free & !Self::NEXT_FREE_CELL_CAN_MERGE;
        self.next_free_raw.set(next_free as *const FreeCell);
    }

    fn next_free(&self) -> *const FreeCell<'a> {
        let next_free = self.next_free_raw.get() as usize & Self::MASK;
        next_free as *const FreeCell<'a>
    }

    unsafe fn from_uninitialized(
        raw: NonNull<u8>,
        size: Bytes,
        next_free: Option<*const FreeCell<'a>>,
        policy: &dyn AllocPolicy<'a>,
    ) -> *const FreeCell<'a> {
        assert_is_word_aligned(raw.as_ptr() as *mut u8);

        let next_free = next_free.unwrap_or(ptr::null_mut());

        let raw = raw.as_ptr() as *mut FreeCell;
        ptr::write(
            raw,
            FreeCell {
                header: CellHeader::default(),
                next_free_raw: Cell::new(next_free),
            },
        );

        write_free_pattern(&*raw, size, policy);

        raw
    }

    fn into_allocated_cell(&self, policy: &dyn AllocPolicy<'a>) -> &AllocatedCell<'a> {
        assert_local_cell_invariants(&self.header);
        assert_is_poisoned_with_free_pattern(self, policy);

        CellHeader::set_allocated(&self.header.neighbors);
        unsafe { mem::transmute(self) }
    }

    // Try and satisfy the given allocation request with this cell.
    fn try_alloc<'b>(
        &'b self,
        previous: &'b Cell<*const FreeCell<'a>>,
        alloc_size: Words,
        align: Bytes,
        policy: &dyn AllocPolicy<'a>,
    ) -> Option<&'b AllocatedCell<'a>> {
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
        // We allocate from the end of this cell, rather than the beginning,
        // because it allows us to satisfy alignment requests. Since we can
        // choose to split at some alignment and return the aligned cell at the
        // end.
        let next = self.header.neighbors.next_unchecked() as usize;
        let split_and_aligned = (next - size.0) & !(align.0 - 1);
        let data = unsafe { self.header.unchecked_data() } as usize;
        let min_cell_size: Bytes = policy.min_cell_size(alloc_size).into();
        if data + size_of::<CellHeader>().0 + min_cell_size.0 <= split_and_aligned {
            let split_cell_head = split_and_aligned - size_of::<CellHeader>().0;
            let split_cell = unsafe {
                &*FreeCell::from_uninitialized(
                    unchecked_unwrap(NonNull::new(split_cell_head as *mut u8)),
                    Bytes(next - split_cell_head) - size_of::<CellHeader>(),
                    None,
                    policy,
                )
            };

            Neighbors::append(&self.header, &split_cell.header);
            self.clear_next_free_can_merge();
            if CellHeader::next_cell_is_invalid(&self.header.neighbors) {
                CellHeader::clear_next_cell_is_invalid(&self.header.neighbors);
                CellHeader::set_next_cell_is_invalid(&split_cell.header.neighbors);
            }

            return Some(split_cell.into_allocated_cell(policy));
        }

        // There isn't enough room to split this cell and still satisfy the
        // requested allocation. Because of the early check, we know this cell
        // is large enough to fit the requested size, but is the cell's data
        // properly aligned?
        if self.header.is_aligned_to(align) {
            previous.set(self.next_free());
            let allocated = self.into_allocated_cell(policy);
            assert_is_valid_free_list(previous.get(), policy);
            return Some(allocated);
        }

        None
    }

    fn insert_into_free_list<'b>(
        &'b self,
        head: &'b Cell<*const FreeCell<'a>>,
        policy: &dyn AllocPolicy<'a>,
    ) -> &'b Cell<*const FreeCell<'a>> {
        extra_assert!(!self.next_free_can_merge());
        extra_assert!(self.next_free().is_null());
        self.next_free_raw.set(head.get());
        head.set(self);
        assert_is_valid_free_list(head.get(), policy);
        head
    }

    #[cfg(feature = "extra_assertions")]
    fn tail_data(&self) -> *const u8 {
        let data = unsafe { (self as *const FreeCell as *const FreeCell).offset(1) as *const u8 };
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

impl<'a> AllocatedCell<'a> {
    unsafe fn into_free_cell(&self, policy: &dyn AllocPolicy<'a>) -> &FreeCell<'a> {
        assert_local_cell_invariants(&self.header);

        CellHeader::set_free(&self.header.neighbors);
        let free: &FreeCell = mem::transmute(self);
        write_free_pattern(free, free.header.size(), policy);
        free.next_free_raw.set(ptr::null_mut());
        free
    }

    fn data(&self) -> *const u8 {
        let cell = &self.header as *const CellHeader;
        assert_local_cell_invariants(cell);
        unsafe { cell.offset(1) as *const u8 }
    }
}

extra_only! {
    fn write_free_pattern(cell: &FreeCell, size: Bytes, policy: &dyn AllocPolicy) {
        unsafe {
            let data = cell.tail_data();
            let pattern = policy.free_pattern();
            ptr::write_bytes(
                data as *mut u8,
                pattern,
                (size - (size_of::<FreeCell>() - size_of::<CellHeader>())).0
            );
        }
    }
}

extra_only! {
    fn assert_is_poisoned_with_free_pattern(cell: &FreeCell, policy: &dyn AllocPolicy) {
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
    fn assert_local_cell_invariants(cell: *const CellHeader) {
        assert_is_word_aligned(cell);
        unsafe {
            if let Some(cell_ref) = cell.as_ref() {
                assert!(cell_ref.size() >= size_of::<usize>());

                if let Some(prev) = cell_ref.neighbors.prev() {
                    assert!(prev.size() >= size_of::<usize>());
                    assert!(!CellHeader::next_cell_is_invalid(&prev.neighbors));
                    assert_eq!(prev.neighbors.next_unchecked(), cell, "next(prev(cell)) == cell");
                }

                if let Some(next) = cell_ref.neighbors.next() {
                    assert!(next.size() >= size_of::<usize>());
                    assert_eq!(next.neighbors.prev_unchecked(), cell, "prev(next(cell)) == cell");
                }

                if let Some(free) = cell_ref.as_free_cell() {
                    if free.next_free_can_merge() {
                        let prev_cell = free.header.neighbors.prev().expect(
                            "if the next free cell (aka prev_cell) can merge, \
                             prev_cell had better exist"
                        );
                        assert!(
                            prev_cell.is_free(),
                            "prev_cell is free, when NEXT_FREE_CELL_CAN_MERGE bit is set"
                        );
                        assert_eq!(
                            free.next_free() as *const CellHeader,
                            prev_cell as *const _,
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
    fn assert_is_valid_free_list(head: *const FreeCell, policy: &dyn AllocPolicy) {
        unsafe {
            let mut left = head;
            assert_local_cell_invariants(left as *const CellHeader);
            if left.is_null() {
                return;
            }
            assert_is_poisoned_with_free_pattern(&*left, policy);

            let mut right = (*left).next_free();

            loop {
                assert_local_cell_invariants(right as *const CellHeader);
                if right.is_null() {
                    return;
                }
                assert_is_poisoned_with_free_pattern(&*right, policy);

                assert!(left != right, "free list should not have cycles");
                assert!((*right).header.is_free(), "cells in free list should never be allocated");
                assert!((*left).header.is_free(), "cells in free list should never be allocated");

                right = (*right).next_free();
                assert_local_cell_invariants(right as *const CellHeader);
                if right.is_null() {
                    return;
                }
                assert_is_poisoned_with_free_pattern(&*right, policy);

                left = (*left).next_free();
                assert_local_cell_invariants(left as *const CellHeader);
                assert_is_poisoned_with_free_pattern(&*left, policy);

                assert!(left != right, "free list should not have cycles");
                assert!((*right).header.is_free(), "cells in free list should never be allocated");
                assert!((*left).header.is_free(), "cells in free list should never be allocated");

                right = (*right).next_free();
            }
        }
    }
}

trait AllocPolicy<'a> {
    unsafe fn new_cell_for_free_list(
        &self,
        size: Words,
        align: Bytes,
    ) -> Result<*const FreeCell<'a>, AllocErr>;

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

impl<'a> AllocPolicy<'a> for LargeAllocPolicy {
    unsafe fn new_cell_for_free_list(
        &self,
        size: Words,
        align: Bytes,
    ) -> Result<*const FreeCell<'a>, AllocErr> {
        // To assure that an allocation will always succeed after refilling the
        // free list with this new cell, make sure that we allocate enough to
        // fulfill the requested alignment, and still have the minimum cell size
        // left over.
        let size: Bytes = cmp::max(size.into(), (align + Self::MIN_CELL_SIZE) * Words(2));

        let pages: Pages = (size + size_of::<CellHeader>()).round_up_to();
        let new_pages = imp::alloc_pages(pages)?;
        let allocated_size: Bytes = pages.into();

        let free_cell = &*FreeCell::from_uninitialized(
            new_pages,
            allocated_size - size_of::<CellHeader>(),
            None,
            self as &dyn AllocPolicy<'a>,
        );

        let next_cell = (new_pages.as_ptr() as *const u8).add(allocated_size.0);
        free_cell
            .header
            .neighbors
            .set_next(next_cell as *const CellHeader);
        CellHeader::set_next_cell_is_invalid(&free_cell.header.neighbors);
        Ok(free_cell)
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

cfg_if! {
    if #[cfg(any(debug_assertions, feature = "extra_assertions"))] {
        unsafe fn unchecked_unwrap<T>(o: Option<T>) -> T {
            o.unwrap()
        }
    } else {
        #[inline]
        unsafe fn unchecked_unwrap<T>(o: Option<T>) -> T {
            match o {
                Some(t) => t,
                None => core::hint::unreachable_unchecked(),
            }
        }
    }
}

unsafe fn walk_free_list<'a, F, T>(
    head: &Cell<*const FreeCell<'a>>,
    policy: &dyn AllocPolicy<'a>,
    mut f: F,
) -> Result<T, AllocErr>
where
    F: FnMut(&Cell<*const FreeCell<'a>>, &FreeCell<'a>) -> Option<T>,
{
    // The previous cell in the free list (not to be confused with the current
    // cell's previously _adjacent_ cell).
    let previous_free = head;

    loop {
        let current_free = previous_free.get();
        assert_local_cell_invariants(&(*current_free).header);

        if current_free.is_null() {
            return Err(AllocErr);
        }

        let current_free = Cell::new(current_free);

        // Now check if this cell can merge with the next cell in the free
        // list.
        //
        // We don't re-check `policy.should_merge_adjacent_free_cells()` because
        // the `NEXT_FREE_CELL_CAN_MERGE` bit only gets set after checking with
        // the policy.
        while (*current_free.get()).next_free_can_merge() {
            extra_assert!(policy.should_merge_adjacent_free_cells());

            let current = &*current_free.get();
            current.clear_next_free_can_merge();

            let prev_neighbor = unchecked_unwrap(
                current
                    .header
                    .neighbors
                    .prev()
                    .and_then(|p| p.as_free_cell()),
            );

            current.header.neighbors.remove();
            if CellHeader::next_cell_is_invalid(&current.header.neighbors) {
                CellHeader::set_next_cell_is_invalid(&prev_neighbor.header.neighbors);
            }

            previous_free.set(prev_neighbor);
            current_free.set(prev_neighbor);

            write_free_pattern(
                &*current_free.get(),
                (*current_free.get()).header.size(),
                policy,
            );
            assert_local_cell_invariants(&(*current_free.get()).header);
        }

        if let Some(result) = f(previous_free, &*current_free.get()) {
            return Ok(result);
        }

        previous_free.set(&*(*current_free.get()).next_free_raw.get());
    }
}

/// Do a first-fit allocation from the given free list.
unsafe fn alloc_first_fit<'a>(
    size: Words,
    align: Bytes,
    head: &Cell<*const FreeCell<'a>>,
    policy: &dyn AllocPolicy<'a>,
) -> Result<NonNull<u8>, AllocErr> {
    extra_assert!(size.0 > 0);

    walk_free_list(head, policy, |previous, current| {
        extra_assert_eq!(previous.get(), current);

        if let Some(allocated) = current.try_alloc(previous, size, align, policy) {
            assert_aligned_to(allocated.data(), align);
            return Some(unchecked_unwrap(NonNull::new(allocated.data() as *mut u8)));
        }

        None
    })
}

unsafe fn alloc_with_refill<'a, 'b>(
    size: Words,
    align: Bytes,
    head: &'b Cell<*const FreeCell<'a>>,
    policy: &dyn AllocPolicy<'a>,
) -> Result<NonNull<u8>, AllocErr> {
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
pub struct WeeAlloc<'a> {
    head: imp::Exclusive<*const FreeCell<'a>>,

    #[cfg(feature = "size_classes")]
    size_classes: size_classes::SizeClasses<'a>,
}

unsafe impl<'a> Sync for WeeAlloc<'a> {}

impl<'a> ConstInit for WeeAlloc<'a> {
    const INIT: WeeAlloc<'a> = WeeAlloc {
        head: imp::Exclusive::INIT,

        #[cfg(feature = "size_classes")]
        size_classes: size_classes::SizeClasses::INIT,
    };
}

impl<'a> WeeAlloc<'a> {
    /// An initial `const` default construction of a `WeeAlloc` allocator.
    ///
    /// This is usable for initializing `static`s that get set as the global
    /// allocator.
    pub const INIT: Self = <Self as ConstInit>::INIT;

    #[cfg(feature = "size_classes")]
    unsafe fn with_free_list_and_policy_for_size<F, T>(&self, size: Words, align: Bytes, f: F) -> T
    where
        F: for<'b> FnOnce(&'b Cell<*const FreeCell<'a>>, &'b dyn AllocPolicy<'a>) -> T,
    {
        extra_assert!(size.0 > 0);
        extra_assert!(align.0 > 0);

        if align <= size_of::<usize>() {
            if let Some(head) = self.size_classes.get(size) {
                let policy = size_classes::SizeClassAllocPolicy(&self.head);
                let policy = &policy as &dyn AllocPolicy<'a>;
                return head.with_exclusive_access(|head| {
                    let head_cell = Cell::new(*head);
                    let result = f(&head_cell, policy);
                    *head = head_cell.get();
                    result
                });
            }
        }

        let policy = &LARGE_ALLOC_POLICY as &dyn AllocPolicy<'a>;
        self.head.with_exclusive_access(|head| {
            let head_cell = Cell::new(*head);
            let result = f(&head_cell, policy);
            *head = head_cell.get();
            result
        })
    }

    #[cfg(not(feature = "size_classes"))]
    unsafe fn with_free_list_and_policy_for_size<F, T>(&self, size: Words, _align: Bytes, f: F) -> T
    where
        F: for<'b> FnOnce(&'b Cell<*const FreeCell<'a>>, &'b dyn AllocPolicy<'a>) -> T,
    {
        extra_assert!(size.0 > 0);
        let policy = &LARGE_ALLOC_POLICY as &dyn AllocPolicy;
        self.head.with_exclusive_access(|head| {
            let head_cell = Cell::new(*head);
            let result = f(&head_cell, policy);
            *head = head_cell.get();
            result
        })
    }

    unsafe fn alloc_impl(&self, layout: Layout) -> Result<NonNull<u8>, AllocErr> {
        let size = Bytes(layout.size());
        let align = if layout.align() == 0 {
            Bytes(1)
        } else {
            Bytes(layout.align())
        };

        if size.0 == 0 {
            // Ensure that our made up pointer is properly aligned by using the
            // alignment as the pointer.
            extra_assert!(align.0 > 0);
            return Ok(NonNull::new_unchecked(align.0 as *mut u8));
        }

        let size: Words = size.round_up_to();

        self.with_free_list_and_policy_for_size(size, align, |head, policy| {
            assert_is_valid_free_list(head.get(), policy);
            alloc_with_refill(size, align, head, policy)
        })
    }

    unsafe fn dealloc_impl(&self, ptr: NonNull<u8>, layout: Layout) {
        let size = Bytes(layout.size());
        if size.0 == 0 {
            return;
        }

        let size: Words = size.round_up_to();
        let align = Bytes(layout.align());

        self.with_free_list_and_policy_for_size(size, align, |head, policy| {
            let cell = (ptr.as_ptr() as *mut CellHeader<'a> as *const CellHeader<'a>).offset(-1);
            let cell = &*cell;

            extra_assert!(cell.size() >= size.into());
            extra_assert!(cell.is_allocated());
            let cell: &AllocatedCell<'a> = mem::transmute(cell);

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

                if let Some(prev) = free
                    .header
                    .neighbors
                    .prev()
                    .and_then(|p| (*p).as_free_cell())
                {
                    free.header.neighbors.remove();
                    if CellHeader::next_cell_is_invalid(&free.header.neighbors) {
                        CellHeader::set_next_cell_is_invalid(&prev.header.neighbors);
                    }

                    write_free_pattern(prev, prev.header.size(), policy);
                    assert_is_valid_free_list(head.get(), policy);
                    return;
                }

                if let Some(next) = free
                    .header
                    .neighbors
                    .next()
                    .and_then(|n| (*n).as_free_cell())
                {
                    free.next_free_raw.set(next.next_free());
                    next.next_free_raw.set(free);
                    next.set_next_free_can_merge();

                    assert_is_valid_free_list(head.get(), policy);
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

#[cfg(feature = "nightly")]
unsafe impl<'a, 'b> Alloc for &'b WeeAlloc<'a>
where
    'a: 'b,
{
    unsafe fn alloc(&mut self, layout: Layout) -> Result<NonNull<u8>, AllocErr> {
        self.alloc_impl(layout)
    }

    unsafe fn dealloc(&mut self, ptr: NonNull<u8>, layout: Layout) {
        self.dealloc_impl(ptr, layout)
    }
}

unsafe impl GlobalAlloc for WeeAlloc<'static> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        match self.alloc_impl(layout) {
            Ok(ptr) => ptr.as_ptr(),
            Err(AllocErr) => ptr::null_mut(),
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if let Some(ptr) = NonNull::new(ptr) {
            self.dealloc_impl(ptr, layout);
        }
    }
}

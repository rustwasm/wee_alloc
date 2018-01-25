//! An example of what using `wee_alloc` as the global allocator in a
//! `#![no_std]` crate targeting `wasm32-unknown-unknown` looks like!

// First, some boilerplate and set up //////////////////////////////////////////

// We aren't using the standard library.
#![no_std]
// Replacing the allocator and using the `alloc` crate are still unstable.
#![feature(alloc, core_intrinsics, global_allocator, lang_items)]

extern crate alloc;
extern crate wee_alloc;

// Use `wee_alloc` as the global allocator.
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

// Need to provide a tiny `panic_fmt` lang-item implementation for
// `#![no_std]`. This will just `trap` if we panic in the resulting WebAssembly.
#[lang = "panic_fmt"]
extern "C" fn panic_fmt(_args: ::core::fmt::Arguments, _file: &'static str, _line: u32) -> ! {
    unsafe {
        ::core::intrinsics::abort();
    }
}

// Now, use the allocator via `alloc` types! ///////////////////////////////////

use alloc::boxed::Box;

// Box a `u8`!
#[no_mangle]
pub extern "C" fn hello() -> *mut u8 {
    Box::into_raw(Box::new(42))
}

// Free a `Box<u8>` that we allocated earlier!
#[no_mangle]
pub unsafe extern "C" fn goodbye(ptr: *mut u8) {
    let _ = Box::from_raw(ptr);
}

use super::{assert_is_word_aligned, PAGE_SIZE};
use const_init::ConstInit;
use core::cell::UnsafeCell;
use units::Pages;

extern {
    #[link_name = "llvm.wasm.current.memory.i32"]
    fn current_memory() -> usize;

    // TODO: this intrinsic actually returns the previous limit, but LLVM
    // doesn't expose that right now. When we upgrade LLVM stop using
    // `current_memory` above. Also handle `-1` as an allocation failure.
    #[link_name = "llvm.wasm.grow.memory.i32"]
    fn grow_memory(pages: usize);
}

unsafe fn get_base_pointer() -> *mut u8 {
    (current_memory() * PAGE_SIZE.0) as _
}

#[inline]
pub(crate) unsafe fn alloc_pages(n: Pages) -> *mut u8 {
    let ptr = get_base_pointer();
    assert_is_word_aligned(ptr);
    extra_assert!(!ptr.is_null());
    grow_memory(n.0);
    ptr
}

pub(crate) struct Exclusive<T> {
    inner: UnsafeCell<T>,

    #[cfg(feature = "extra_assertions")]
    in_use: bool,
}

impl<T: ConstInit> ConstInit for Exclusive<T> {
    const INIT: Self = Exclusive {
        inner: UnsafeCell::new(T::INIT),

        #[cfg(feature = "extra_assertions")]
        in_use: false,
    };
}

extra_only! {
    fn assert_not_in_use<T>(excl: &Exclusive<T>) {
        extra_assert!(!excl.in_use, "`Exclusive<T>` is not re-entrant");
    }
}

impl<T> Exclusive<T> {
    /// Get exclusive, mutable access to the inner value.
    ///
    /// # Safety
    ///
    /// It is the callers' responsibility to ensure that `f` does not re-enter
    /// this method for this `Exclusive` instance.
    #[inline]
    pub(crate) unsafe fn with_exclusive_access<'a, F, U>(&'a self, f: F) -> U
    where
        F: FnOnce(&'a mut T) -> U
    {
        assert_not_in_use(self);
        f(&mut *self.inner.get())
    }
}

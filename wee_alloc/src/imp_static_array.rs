use super::AllocError;
use const_init::ConstInit;
#[cfg(feature = "extra_assertions")]
use core::cell::Cell;
use core::ptr::NonNull;
use memory_units::{Bytes, Pages};
use spin::Mutex;

const SCRATCH_LEN_BYTES: usize = include!(concat!(
    env!("OUT_DIR"),
    "/wee_alloc_static_array_backend_size_bytes.txt"
));

#[repr(align(4096))]
struct ScratchHeap([u8; SCRATCH_LEN_BYTES]);

static mut SCRATCH_HEAP: ScratchHeap = ScratchHeap([0; SCRATCH_LEN_BYTES]);
static mut OFFSET: Mutex<usize> = Mutex::new(0);

pub(crate) unsafe fn alloc_pages(pages: Pages) -> Result<NonNull<u8>, AllocError> {
    let bytes: Bytes = pages.into();
    let mut offset = OFFSET.lock();
    let end = bytes.0.checked_add(*offset).ok_or(AllocError)?;
    if end < SCRATCH_LEN_BYTES {
        let ptr = SCRATCH_HEAP.0[*offset..end].as_mut_ptr() as *mut u8;
        *offset = end;
        NonNull::new(ptr).ok_or_else(|| AllocError)
    } else {
        Err(AllocError)
    }
}

pub(crate) struct Exclusive<T> {
    inner: Mutex<T>,

    #[cfg(feature = "extra_assertions")]
    in_use: Cell<bool>,
}

impl<T: ConstInit> ConstInit for Exclusive<T> {
    const INIT: Self = Exclusive {
        inner: Mutex::new(T::INIT),

        #[cfg(feature = "extra_assertions")]
        in_use: Cell::new(false),
    };
}

extra_only! {
    fn assert_not_in_use<T>(excl: &Exclusive<T>) {
        assert!(!excl.in_use.get(), "`Exclusive<T>` is not re-entrant");
    }
}

extra_only! {
    fn set_in_use<T>(excl: &Exclusive<T>) {
        excl.in_use.set(true);
    }
}

extra_only! {
    fn set_not_in_use<T>(excl: &Exclusive<T>) {
        excl.in_use.set(false);
    }
}

impl<T> Exclusive<T> {
    /// Get exclusive, mutable access to the inner value.
    ///
    /// # Safety
    ///
    /// It is the callers' responsibility to ensure that `f` does not re-enter
    /// this method for this `Exclusive` instance.
    //
    // XXX: If we don't mark this function inline, then it won't be, and the
    // code size also blows up by about 200 bytes.
    #[inline]
    pub(crate) unsafe fn with_exclusive_access<'a, F, U>(&'a self, f: F) -> U
    where
        for<'x> F: FnOnce(&'x mut T) -> U,
    {
        let mut guard = self.inner.lock();
        assert_not_in_use(self);
        set_in_use(self);
        let result = f(&mut guard);
        set_not_in_use(self);
        result
    }
}

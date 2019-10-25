use super::AllocErr;
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
struct ScratchHeap([u8; SCRATCH_LEN_BYTES]);// = [0; SCRATCH_LEN_BYTES]);

static mut SCRATCH_HEAP: ScratchHeap = ScratchHeap([0; SCRATCH_LEN_BYTES]);

//static mut SCRATCH_HEAP: [u8; SCRATCH_LEN_BYTES] = [0; SCRATCH_LEN_BYTES];
static mut OFFSET: Mutex<usize> = Mutex::new(0);


pub(crate) unsafe fn alloc_pages(
    pages: Pages,
    align: Bytes,
) -> Result<NonNull<u8>, AllocErr> {
    let bytes: Bytes = pages.into();
    let offset = OFFSET.lock();

    let scratch_heap_start = (SCRATCH_HEAP.0).as_mut_ptr() as usize;
    let scratch_heap_end = scratch_heap_start + (SCRATCH_HEAP.0).len();
    let unaligned_end = scratch_heap_start + *offset;
    let aligned_end = round_up_to_alignment(unaligned_end, align.0);
    // NB: `unaligned_ptr <= aligned_ptr` handles potential overflow in `round_up_to_alignment`.
    if unaligned_end <= aligned_end && aligned_end < scratch_heap_end
        && aligned_end.checked_add(bytes.0).ok_or(AllocErr)? < scratch_heap_end {

        let aligned_ptr = (SCRATCH_HEAP.0)[*offset..aligned_end].as_mut_ptr() as *mut u8;
        *offset = aligned_end;
        NonNull::new(aligned_ptr).ok_or_else(|| AllocErr)
    } else {
        Err(AllocErr)
    }
}

fn round_up_to_alignment(n: usize, align: usize) -> usize {
    extra_assert!(align > 0);
    extra_assert!(align.is_power_of_two());
    (n + align - 1) & !(align - 1)
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

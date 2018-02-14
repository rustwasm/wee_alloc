use const_init::ConstInit;
use core::cell::UnsafeCell;
use memory_units::{Bytes, Pages};

use winapi::shared::minwindef::FALSE;
use winapi::shared::ntdef::NULL;
use winapi::um::memoryapi::VirtualAlloc;
use winapi::um::minwinbase::SECURITY_ATTRIBUTES;
use winapi::um::synchapi::{CreateMutexW, ReleaseMutex, WaitForSingleObject};
use winapi::um::winbase::{WAIT_OBJECT_0, INFINITE};
use winapi::um::winnt::{HANDLE, MEM_COMMIT, PAGE_READWRITE};

pub(crate) fn alloc_pages(pages: Pages) -> *mut u8 {
    let bytes: Bytes = pages.into();
    unsafe { VirtualAlloc(NULL, bytes.0, MEM_COMMIT, PAGE_READWRITE) as *mut u8 }
}

// Cache line size on an i7. Good enough.
const CACHE_LINE_SIZE: usize = 64;

pub(crate) struct Exclusive<T> {
    lock: UnsafeCell<HANDLE>,
    inner: UnsafeCell<T>,
    // Because we can't do `repr(align = "64")` yet, we have to pad a full cache
    // line to ensure that there is no false sharing.
    _no_false_sharing: [u8; CACHE_LINE_SIZE],
}

impl<T: ConstInit> ConstInit for Exclusive<T> {
    const INIT: Self = Exclusive {
        lock: UnsafeCell::new(NULL),
        inner: UnsafeCell::new(T::INIT),
        _no_false_sharing: [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0,
        ],
    };
}

impl<T> Exclusive<T> {
    /// Get exclusive, mutable access to the inner value.
    ///
    /// # Safety
    ///
    /// Does not assert that the mutex calls return OK, unless the
    /// "extra_assertions" feature is enabled. This means that if `f` re-enters
    /// this method for the same `Exclusive` instance, there will be undetected
    /// mutable aliasing, which is UB.
    #[inline]
    pub(crate) unsafe fn with_exclusive_access<'a, F, U>(&'a self, f: F) -> U
    where
        F: FnOnce(&'a mut T) -> U,
    {
        // If we haven't been through here yet, initialize the mutex.
        if *self.lock.get() == NULL {
            *self.lock.get() =
                CreateMutexW(NULL as *mut SECURITY_ATTRIBUTES, FALSE, NULL as *mut u16);
            extra_assert!(*self.lock.get() != NULL);
        }

        let code = WaitForSingleObject(*self.lock.get(), INFINITE);
        extra_assert_eq!(
            code,
            WAIT_OBJECT_0,
            "WaitForSingleObject should return WAIT_OBJECT_0"
        );

        let result = f(&mut *self.inner.get());

        let code = ReleaseMutex(*self.lock.get());
        extra_assert!(code != 0, "ReleaseMutex should return nonzero");

        result
    }
}

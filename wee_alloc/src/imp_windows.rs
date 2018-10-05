use const_init::ConstInit;
use super::AllocErr;
use core::cell::UnsafeCell;
use core::ptr::NonNull;
use memory_units::{Bytes, Pages};

use winapi::shared::ntdef::NULL;
use winapi::um::memoryapi::VirtualAlloc;
use winapi::um::synchapi::{
    SRWLOCK, SRWLOCK_INIT, AcquireSRWLockExclusive, ReleaseSRWLockExclusive,
};
use winapi::um::winnt::{MEM_COMMIT, PAGE_READWRITE};

pub(crate) fn alloc_pages(pages: Pages) -> Result<NonNull<u8>, AllocErr> {
    let bytes: Bytes = pages.into();
    let ptr = unsafe { VirtualAlloc(NULL, bytes.0, MEM_COMMIT, PAGE_READWRITE) };
    NonNull::new(ptr as *mut u8).ok_or(AllocErr)
}

// Align to the cache line size on an i7 to avoid false sharing.
#[repr(align(64))]
pub(crate) struct Exclusive<T> {
    lock: UnsafeCell<SRWLOCK>,
    inner: UnsafeCell<T>,
}

impl<T: ConstInit> ConstInit for Exclusive<T> {
    const INIT: Self = Exclusive {
        lock: UnsafeCell::new(SRWLOCK_INIT),
        inner: UnsafeCell::new(T::INIT),
    };
}

impl<T> Exclusive<T> {
    /// Get exclusive, mutable access to the inner value.
    #[inline]
    pub(crate) unsafe fn with_exclusive_access<'a, F, U>(&'a self, f: F) -> U
    where
        F: FnOnce(&'a mut T) -> U,
    {
        AcquireSRWLockExclusive(self.lock.get());

        let result = f(&mut *self.inner.get());

        ReleaseSRWLockExclusive(self.lock.get());

        result
    }
}

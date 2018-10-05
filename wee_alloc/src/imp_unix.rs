use super::AllocErr;
use const_init::ConstInit;
use core::cell::UnsafeCell;
use core::ptr;
use libc;
use memory_units::{Bytes, Pages};

pub(crate) fn alloc_pages(pages: Pages) -> Result<ptr::NonNull<u8>, AllocErr> {
    unsafe {
        let bytes: Bytes = pages.into();
        let addr = libc::mmap(
            ptr::null_mut(),
            bytes.0,
            libc::PROT_WRITE | libc::PROT_READ,
            libc::MAP_ANON | libc::MAP_PRIVATE,
            -1,
            0,
        );
        if addr == libc::MAP_FAILED {
            Err(AllocErr)
        } else {
            ptr::NonNull::new(addr as *mut u8).ok_or(AllocErr)
        }
    }
}

// Align to the cache line size on an i7 to prevent false sharing.
#[repr(align(64))]
pub(crate) struct Exclusive<T> {
    lock: UnsafeCell<libc::pthread_mutex_t>,
    inner: UnsafeCell<T>,
}

impl<T: ConstInit> ConstInit for Exclusive<T> {
    const INIT: Self = Exclusive {
        lock: UnsafeCell::new(libc::PTHREAD_MUTEX_INITIALIZER),
        inner: UnsafeCell::new(T::INIT),
    };
}

impl<T> Exclusive<T> {
    /// Get exclusive, mutable access to the inner value.
    ///
    /// # Safety
    ///
    /// Does not assert that `pthread`s calls return OK, unless the
    /// "extra_assertions" feature is enabled. This means that if `f` re-enters
    /// this method for the same `Exclusive` instance, there will be undetected
    /// mutable aliasing, which is UB.
    #[inline]
    pub(crate) unsafe fn with_exclusive_access<F, U>(&self, f: F) -> U
    where
        for<'x> F: FnOnce(&'x mut T) -> U,
    {
        let code = libc::pthread_mutex_lock(&mut *self.lock.get());
        extra_assert_eq!(code, 0, "pthread_mutex_lock should run OK");

        let result = f(&mut *self.inner.get());

        let code = libc::pthread_mutex_unlock(&mut *self.lock.get());
        extra_assert_eq!(code, 0, "pthread_mutex_unlock should run OK");

        result
    }
}

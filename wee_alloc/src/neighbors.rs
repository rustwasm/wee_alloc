//! An intrusive, doubly-linked list of adjacent cells.

use core::cell::Cell;
use core::marker::PhantomData;
use core::ptr;

/// TODO FITZGEN
///
/// ### Safety
///
/// TODO FITZGEN
pub unsafe trait HasNeighbors<'a, T>: AsRef<Neighbors<'a, T>>
where
    T: 'a + HasNeighbors<'a, T>,
{
    unsafe fn next_checked(neighbors: &Neighbors<'a, T>, next: *const T) -> Option<&'a T>;
    unsafe fn prev_checked(neighbors: &Neighbors<'a, T>, prev: *const T) -> Option<&'a T>;
}

#[derive(Debug)]
pub struct Neighbors<'a, T>
where
    T: 'a + HasNeighbors<'a, T>,
{
    next_raw: Cell<*const T>,
    prev_raw: Cell<*const T>,
    _phantom: PhantomData<&'a T>,
}

impl<'a, T> Default for Neighbors<'a, T>
where
    T: 'a + HasNeighbors<'a, T>,
{
    fn default() -> Self {
        Neighbors {
            next_raw: Cell::new(ptr::null_mut()),
            prev_raw: Cell::new(ptr::null_mut()),
            _phantom: PhantomData,
        }
    }
}

// Add this `cfg` so that the build will break on platforms with bizarre word
// sizes, where we might not have acceess to these low bits.
#[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
impl<'a, T> Neighbors<'a, T>
where
    T: 'a + HasNeighbors<'a, T>,
{
    // We use two low bits from each of our pointers.
    pub const BIT_1: usize = 0b01;
    pub const BIT_2: usize = 0b10;

    // Mask to get just the low bits.
    const BITS_MASK: usize = 0b11;

    // Mask to get the aligned pointer.
    const PTR_MASK: usize = !0b11;
}

#[test]
fn can_use_low_bits() {
    use core::mem;
    assert!(
        mem::align_of::<*const u8>() >= 0b100,
        "we rely on being able to stick tags into the lowest two bits"
    );
}

/// Get bits.
#[allow(dead_code)]
impl<'a, T> Neighbors<'a, T>
where
    T: 'a + HasNeighbors<'a, T>,
{
    #[inline]
    pub fn get_next_bit_1(&self) -> bool {
        self.next_raw.get() as usize & Self::BIT_1 != 0
    }

    #[inline]
    pub fn get_next_bit_2(&self) -> bool {
        self.next_raw.get() as usize & Self::BIT_2 != 0
    }

    #[inline]
    pub fn get_prev_bit_1(&self) -> bool {
        self.prev_raw.get() as usize & Self::BIT_1 != 0
    }

    #[inline]
    pub fn get_prev_bit_2(&self) -> bool {
        self.prev_raw.get() as usize & Self::BIT_2 != 0
    }
}

/// Set bits.
#[allow(dead_code)]
impl<'a, T> Neighbors<'a, T>
where
    T: 'a + HasNeighbors<'a, T>,
{
    #[inline]
    pub fn set_next_bit_1(&self) {
        let next_raw = self.next_raw.get() as usize;
        let next_raw = next_raw | Self::BIT_1;
        self.next_raw.set(next_raw as *const T);
    }

    #[inline]
    pub fn set_next_bit_2(&self) {
        let next_raw = self.next_raw.get() as usize;
        let next_raw = next_raw | Self::BIT_2;
        self.next_raw.set(next_raw as *const T);
    }

    #[inline]
    pub fn set_prev_bit_1(&self) {
        let prev_raw = self.prev_raw.get() as usize;
        let prev_raw = prev_raw | Self::BIT_1;
        self.prev_raw.set(prev_raw as *const T);
    }

    #[inline]
    pub fn set_prev_bit_2(&self) {
        let prev_raw = self.prev_raw.get() as usize;
        let prev_raw = prev_raw | Self::BIT_2;
        self.prev_raw.set(prev_raw as *const T);
    }
}

/// Clear bits.
#[allow(dead_code)]
impl<'a, T> Neighbors<'a, T>
where
    T: 'a + HasNeighbors<'a, T>,
{
    #[inline]
    pub fn clear_next_bit_1(&self) {
        let next_raw = self.next_raw.get() as usize;
        let next_raw = next_raw & !Self::BIT_1;
        self.next_raw.set(next_raw as *const T);
    }

    #[inline]
    pub fn clear_next_bit_2(&self) {
        let next_raw = self.next_raw.get() as usize;
        let next_raw = next_raw & !Self::BIT_2;
        self.next_raw.set(next_raw as *const T);
    }

    #[inline]
    pub fn clear_prev_bit_1(&self) {
        let prev_raw = self.prev_raw.get() as usize;
        let prev_raw = prev_raw & !Self::BIT_1;
        self.prev_raw.set(prev_raw as *const T);
    }

    #[inline]
    pub fn clear_prev_bit_2(&self) {
        let prev_raw = self.prev_raw.get() as usize;
        let prev_raw = prev_raw & !Self::BIT_2;
        self.prev_raw.set(prev_raw as *const T);
    }
}

/// Get pointers.
impl<'a, T> Neighbors<'a, T>
where
    T: 'a + HasNeighbors<'a, T>,
{
    #[inline]
    pub fn next_unchecked(&self) -> *const T {
        let next = self.next_raw.get() as usize;
        let next = next & Self::PTR_MASK;
        next as *const T
    }

    #[inline]
    pub fn prev_unchecked(&self) -> *const T {
        let prev = self.prev_raw.get() as usize;
        let prev = prev & Self::PTR_MASK;
        prev as *const T
    }

    #[inline]
    pub fn next(&self) -> Option<&'a T> {
        unsafe { T::next_checked(self, self.next_unchecked()) }
    }

    #[inline]
    pub fn prev(&self) -> Option<&'a T> {
        unsafe { T::prev_checked(self, self.prev_unchecked()) }
    }
}

/// Sibling pointer setters that don't attempt to make sure the doubly-linked
/// list is well-formed. The pointers are required to be aligned, however, and
/// the low bits are not clobbered.
impl<'a, T> Neighbors<'a, T>
where
    T: 'a + HasNeighbors<'a, T>,
{
    #[inline]
    pub unsafe fn set_next(&self, next: *const T) {
        let next = next as usize;
        extra_assert_eq!(next & Self::BITS_MASK, 0);
        let old_next = self.next_raw.get() as usize;
        let old_bits = old_next & Self::BITS_MASK;
        let next = next | old_bits;
        self.next_raw.set(next as *const T);
    }

    #[inline]
    pub unsafe fn set_prev(&self, prev: *const T) {
        let prev = prev as usize;
        extra_assert_eq!(prev & Self::BITS_MASK, 0);
        let old_prev = self.prev_raw.get() as usize;
        let old_bits = old_prev & Self::BITS_MASK;
        let prev = prev | old_bits;
        self.prev_raw.set(prev as *const T);
    }
}

/// Raw sibling pointer getters that include the lower bits too, if any are set.
#[allow(dead_code)]
impl<'a, T> Neighbors<'a, T>
where
    T: 'a + HasNeighbors<'a, T>,
{
    #[inline]
    pub unsafe fn next_and_bits(&self) -> *const T {
        self.next_raw.get()
    }

    #[inline]
    pub unsafe fn prev_and_bits(&self) -> *const T {
        self.prev_raw.get()
    }
}

/// Raw sibling pointer setters that clobber the lower bits too.
#[allow(dead_code)]
impl<'a, T> Neighbors<'a, T>
where
    T: 'a + HasNeighbors<'a, T>,
{
    #[inline]
    pub unsafe fn set_next_and_bits(&self, next_and_bits: *const T) {
        self.next_raw.set(next_and_bits);
    }

    #[inline]
    pub unsafe fn set_prev_and_bits(&self, prev_and_bits: *const T) {
        self.prev_raw.set(prev_and_bits);
    }
}

/// Higher level list manipulations.
///
/// These do not modify or propagate any bits; that is the caller's
/// responsibility.
impl<'a, T> Neighbors<'a, T>
where
    T: 'a + HasNeighbors<'a, T>,
{
    #[inline]
    pub fn remove(&self) {
        unsafe {
            if let Some(next) = self.next() {
                next.as_ref().set_prev(self.prev_unchecked());
            }

            if let Some(prev) = self.prev() {
                prev.as_ref().set_next(self.next_unchecked());
            }

            self.set_next(ptr::null_mut());
            self.set_prev(ptr::null_mut());
        }
    }

    #[inline]
    pub fn append(me: &T, neighbor: &T) {
        extra_assert!(neighbor.as_ref().next_unchecked().is_null());
        extra_assert!(neighbor.as_ref().prev_unchecked().is_null());

        unsafe {
            neighbor.as_ref().set_next(me.as_ref().next_unchecked());
            if let Some(next) = me.as_ref().next() {
                next.as_ref().set_prev(neighbor);
            }

            neighbor.as_ref().set_prev(me);
            me.as_ref().set_next(neighbor);
        }
    }
}

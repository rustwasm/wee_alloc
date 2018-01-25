use super::{alloc_with_refill, AllocPolicy, Cell, LargeAllocPolicy};
use const_init::ConstInit;
use core::cmp;
use imp;
use units::{Bytes, size_of, RoundUpTo, Words};

/// An array of free lists specialized for allocations of sizes
/// `1..Self::NUM_SIZE_CLASSES + 1` words.
pub(crate) struct SizeClasses(pub(crate) [imp::Exclusive<*mut Cell>; SizeClasses::NUM_SIZE_CLASSES]);

impl ConstInit for SizeClasses {
    const INIT: SizeClasses = SizeClasses(include!("size_classes_init.rs"));
}

impl SizeClasses {
    pub(crate) const NUM_SIZE_CLASSES: usize = 256;

    pub(crate) fn get(&self, size: Words) -> Option<&imp::Exclusive<*mut Cell>> {
        extra_assert!(size.0 > 0);
        self.0.get(size.0 - 1)
    }
}

// The minimum segment size the `SizeClassAllocPolicy` should get from the
// `LargeAllocPolicy`.
const MIN_NEW_CELL_SIZE: Bytes = Bytes(8192);

pub(crate) struct SizeClassAllocPolicy<'a>(pub(crate) &'a imp::Exclusive<*mut Cell>);

impl<'a> AllocPolicy for SizeClassAllocPolicy<'a> {
    unsafe fn new_cell_for_free_list(&self, size: Words) -> Result<*mut Cell, ()> {
        let new_cell_size = cmp::max(size * size, MIN_NEW_CELL_SIZE.round_up_to());

        let new_cell = self.0.with_exclusive_access(|head| {
            alloc_with_refill(new_cell_size, head, &LargeAllocPolicy)
        })?;
        let new_cell = new_cell as *mut Cell;

        let new_cell_size: Bytes = new_cell_size.into();
        Cell::write_initial(new_cell_size - size_of::<Cell>(), new_cell);

        Ok(new_cell)
    }

    fn min_cell_size(&self, alloc_size: Words) -> Words {
        alloc_size
    }

    #[cfg(feature = "extra_assertions")]
    fn allocated_sentinel(&self) -> *mut Cell {
        Cell::SIZE_CLASS_ALLOCATED_NEXT
    }

    #[cfg(feature = "extra_assertions")]
    fn free_pattern(&self) -> u8 {
        Cell::SIZE_CLASS_FREE_PATTERN
    }
}

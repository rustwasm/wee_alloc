use super::{alloc_with_refill, AllocErr, AllocPolicy, CellHeader, FreeCell, LargeAllocPolicy};
use const_init::ConstInit;
use core::cell::Cell;
use core::cmp;
use imp;
use memory_units::{size_of, Bytes, RoundUpTo, Words};

/// An array of free lists specialized for allocations of sizes
/// `1..Self::NUM_SIZE_CLASSES + 1` words.
pub(crate) struct SizeClasses<'a>(
    pub(crate) [imp::Exclusive<*const FreeCell<'a>>; SizeClasses::NUM_SIZE_CLASSES],
);

impl<'a> ConstInit for SizeClasses<'a> {
    const INIT: SizeClasses<'a> = SizeClasses(include!("size_classes_init.rs"));
}

impl<'a> SizeClasses<'a> {
    pub(crate) const NUM_SIZE_CLASSES: usize = 256;

    pub(crate) fn get(&self, size: Words) -> Option<&imp::Exclusive<*const FreeCell<'a>>> {
        extra_assert!(size.0 > 0);
        self.0.get(size.0 - 1)
    }
}

// The minimum segment size the `SizeClassAllocPolicy` should get from the
// `LargeAllocPolicy`.
const MIN_NEW_CELL_SIZE: Bytes = Bytes(8192);

pub(crate) struct SizeClassAllocPolicy<'a, 'b>(pub(crate) &'b imp::Exclusive<*const FreeCell<'a>>)
where
    'a: 'b;

impl<'a, 'b> AllocPolicy<'a> for SizeClassAllocPolicy<'a, 'b>
where
    'a: 'b,
{
    unsafe fn new_cell_for_free_list(
        &self,
        size: Words,
        align: Bytes,
    ) -> Result<*const FreeCell<'a>, AllocErr> {
        extra_assert!(align.0 > 0);
        extra_assert!(align.0.is_power_of_two());
        extra_assert!(align <= size_of::<usize>());

        // Need room for at least size^2 allocations.
        let size_of_header: Words = size_of::<CellHeader>().round_up_to();
        let size_with_header = size + size_of_header;
        let new_cell_size = cmp::max(
            size_with_header * size_with_header,
            MIN_NEW_CELL_SIZE.round_up_to(),
        );

        let new_cell = self.0.with_exclusive_access(|head| {
            let head_cell = Cell::new(*head);
            let result = alloc_with_refill(
                new_cell_size,
                size_of::<usize>(),
                &head_cell,
                &LargeAllocPolicy,
            );
            *head = head_cell.get();
            result
        })?;

        let new_cell_size: Bytes = new_cell_size.into();

        let free_cell = FreeCell::from_uninitialized(
            new_cell,
            new_cell_size - size_of::<CellHeader>(),
            None,
            self as &dyn AllocPolicy,
        );
        let next_cell = (new_cell.as_ptr() as *const u8).offset(new_cell_size.0 as isize);
        (*free_cell)
            .header
            .neighbors
            .set_next(next_cell as *const CellHeader);
        CellHeader::set_next_cell_is_invalid(&(*free_cell).header.neighbors);
        Ok(free_cell)
    }

    fn min_cell_size(&self, alloc_size: Words) -> Words {
        alloc_size
    }

    fn should_merge_adjacent_free_cells(&self) -> bool {
        // It doesn't make sense to merge cells back together when we know it
        // won't enable satisfying larger requests. There won't be any larger
        // requests, because we only allocate for a single size. If we merged
        // cells, they would just split again on the next allocation.
        false
    }

    #[cfg(feature = "extra_assertions")]
    fn free_pattern(&self) -> u8 {
        CellHeader::SIZE_CLASS_FREE_PATTERN
    }
}

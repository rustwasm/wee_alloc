use super::PAGE_SIZE;
use core::mem;
use core::ops;

#[inline]
pub fn size_of<T>() -> Bytes {
    Bytes(mem::size_of::<T>())
}

#[inline]
fn round_up_to(n: usize, divisor: usize) -> usize {
    extra_assert!(divisor > 0);
    (n + divisor - 1) / divisor
}

pub trait RoundUpTo<T> {
    fn round_up_to(self) -> T;
}

macro_rules! define_unit_type {
    ( $name:ident ) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        pub struct $name(pub usize);

        impl<T: Into<Self>> ops::Add<T> for $name {
            type Output = Self;

            #[inline]
            fn add(self, rhs: T) -> Self {
                $name(self.0 + rhs.into().0)
            }
        }

        impl<T: Into<Self>> ops::Sub<T> for $name {
            type Output = Self;

            #[inline]
            fn sub(self, rhs: T) -> Self {
                $name(self.0 - rhs.into().0)
            }
        }

        impl<T: Into<Self>> ops::Mul<T> for $name {
            type Output = Self;

            #[inline]
            fn mul(self, rhs: T) -> Self {
                $name(self.0 * rhs.into().0)
            }
        }

        impl<T: Into<Self>> ops::Div<T> for $name {
            type Output = Self;

            #[inline]
            fn div(self, rhs: T) -> Self {
                $name(self.0 / rhs.into().0)
            }
        }
    }
}

define_unit_type!(Bytes);
define_unit_type!(Words);
define_unit_type!(Pages);

impl From<Words> for Bytes {
    #[inline]
    fn from(words: Words) -> Bytes {
        Bytes(words.0 * mem::size_of::<usize>())
    }
}

impl From<Pages> for Bytes {
    #[inline]
    fn from(pages: Pages) -> Bytes {
        Bytes(pages.0 * PAGE_SIZE.0)
    }
}

impl RoundUpTo<Words> for Bytes {
    #[inline]
    fn round_up_to(self) -> Words {
        Words(round_up_to(self.0, mem::size_of::<usize>()))
    }
}

impl RoundUpTo<Pages> for Bytes {
    #[inline]
    fn round_up_to(self) -> Pages {
        Pages(round_up_to(self.0, PAGE_SIZE.0))
    }
}

impl From<Pages> for Words {
    #[inline]
    fn from(pages: Pages) -> Words {
        Words(pages.0 * PAGE_SIZE.0 / mem::size_of::<usize>())
    }
}

impl RoundUpTo<Pages> for Words {
    #[inline]
    fn round_up_to(self) -> Pages {
        let bytes: Bytes = self.into();
        bytes.round_up_to()
    }
}

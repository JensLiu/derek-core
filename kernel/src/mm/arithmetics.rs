use super::{layout::PAGE_ORDER, memory::PhysAddr};

#[inline]
#[allow(non_snake_case)]
pub const fn ALIGN_UP(val: usize, order: usize) -> usize {
    let o = (1usize << order) - 1;
    // o = 0..0_1111_1111_1111 = 4095
    // !o = 1..1_0000_0000_0000
    // & !o: setting the last 12-bits to zero
    (val + o) & !o
}

#[inline]
#[allow(non_snake_case)]
pub const fn ALIGN_DOWN(val: usize, order: usize) -> usize {
    val & !((1usize << order) - 1)
}

#[inline]
#[allow(non_snake_case)]
pub const fn PG_ROUND_DOWN(val: usize) -> usize {
    ALIGN_DOWN(val, PAGE_ORDER)
}

#[inline]
#[allow(non_snake_case)]
pub const fn PG_ROUND_UP(val: usize) -> usize {
    ALIGN_UP(val, PAGE_ORDER)
}

#[inline]
#[allow(non_snake_case)]
pub fn PA2PPN(pa: usize) -> usize {
    (pa >> 12) << 10
}

#[inline]
#[allow(non_snake_case)]
pub fn PTE2PA(pte: usize) -> usize {
    (pte >> 10) << 12
}

// --------------- Arithmetic Properties --------------

#[macro_export]
macro_rules! impl_address_arithmetics {
    ($struct_name: ident) => {
        impl $struct_name {
            pub fn is_page_aligned(&self) -> bool {
                self.0 % PAGE_SIZE == 0
            }
            pub fn align_down(self) -> Self {
                Self(PG_ROUND_DOWN(self.0))
            }
            pub fn align_up(self) -> Self {
                Self(PG_ROUND_UP(self.0))
            }
            pub fn as_usize(&self) -> usize {
                self.0
            }
            pub fn into_usize(self) -> usize {
                self.0
            }
            /// Get VPN or PPN corresponding to the address
            pub fn get_number(self, page_size: usize) -> usize {
                self.0 / page_size
            }
        }

        impl Add for $struct_name {
            type Output = $struct_name;

            fn add(self, rhs: Self) -> Self::Output {
                Self(self.0 + rhs.0)
            }
        }

        impl Sub for $struct_name {
            type Output = usize;

            fn sub(self, rhs: Self) -> Self::Output {
                self.0 - rhs.0
            }
        }

        impl Add<usize> for $struct_name {
            type Output = $struct_name;

            fn add(self, rhs: usize) -> Self::Output {
                Self(self.0 + rhs)
            }
        }

        impl Sub<usize> for $struct_name {
            type Output = usize;

            fn sub(self, rhs: usize) -> Self::Output {
                self.0 - rhs
            }
        }
    };
}

// ---------------- Range Object -------------------------

/// used for iterator
pub trait StepByOne {
    fn step_one(&mut self);
}

/// We use this to represent a range of values
#[derive(Clone, Copy, Debug)]
pub struct SimpleRange<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd,
{
    begin: T,
    end: T,
}

impl<T> SimpleRange<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd,
{
    pub fn new(begin: T, end: T) -> Self {
        Self { begin, end }
    }

    pub fn get_begin(&self) -> T {
        self.begin
    }

    pub fn get_end(&self) -> T {
        self.end
    }

    // TODO: is this a good design?
    pub fn iter(&self) -> SimpleRangeIterator<T> {
        SimpleRangeIterator {
            current: self.begin,
            end: self.end,
        }
    }
}

/// Trait that converts `SimpleRange` into an iterator
impl<T> IntoIterator for SimpleRange<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd,
{
    type Item = T;

    type IntoIter = SimpleRangeIterator<T>;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            current: self.begin,
            end: self.end,
        }
    }
}

/// An iterator for simple range
/// `StepByOne` is the trait that advances `T` for a unit
pub struct SimpleRangeIterator<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd,
{
    current: T,
    end: T,
}

impl<T> Iterator for SimpleRangeIterator<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current == self.end {
            None
        } else {
            let ret = self.current; // Since T impls Copy, it is copied
            self.current.step_one();
            Some(ret)
        }
    }
}

pub fn arithmetics_done_right() {
    {
        let pa = PhysAddr::new(1);
        let pa1 = pa;
        assert_eq!(pa.align_down(), PhysAddr::new(0));
        assert_eq!(pa1.align_up(), PhysAddr::new(4096));
    }
}

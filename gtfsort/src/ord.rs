use std::{borrow::Cow, cmp::Ordering, fmt::Debug, ops::Deref};

#[derive(Debug, PartialEq, Eq)]
pub struct CowNaturalSort<'a>(pub Cow<'a, str>);

impl<'a> CowNaturalSort<'a> {
    /// Wraps borrowed or owned text for natural ordering.
    #[inline(always)]
    pub fn new(s: Cow<'a, str>) -> Self {
        Self(s)
    }
}

impl Deref for CowNaturalSort<'_> {
    type Target = str;

    /// Borrows the wrapped text as a string slice.
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PartialOrd for CowNaturalSort<'_> {
    /// Delegates partial comparison to the total natural ordering.
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CowNaturalSort<'_> {
    /// Compares strings using human-friendly natural ordering.
    #[inline(always)]
    fn cmp(&self, other: &Self) -> Ordering {
        natord::compare(&self.0, &other.0)
    }
}

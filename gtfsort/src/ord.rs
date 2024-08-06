use std::{borrow::Cow, cmp::Ordering, fmt::Debug, ops::Deref};

#[derive(Debug, PartialEq, Eq)]
pub struct CowNaturalSort<'a>(pub Cow<'a, str>);

impl<'a> CowNaturalSort<'a> {
    #[inline(always)]
    pub fn new(s: Cow<'a, str>) -> Self {
        Self(s)
    }
}

impl Deref for CowNaturalSort<'_> {
    type Target = str;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PartialOrd for CowNaturalSort<'_> {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CowNaturalSort<'_> {
    #[inline(always)]
    fn cmp(&self, other: &Self) -> Ordering {
        natord::compare(&self.0, &other.0)
    }
}

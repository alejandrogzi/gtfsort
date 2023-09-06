use std::cmp::Ordering;
use natord::compare;

#[derive(Debug, PartialEq, Eq, PartialOrd)]
pub struct Sort {
    value: String,
}

impl Sort {
    pub fn new(value: &str) -> Self {
        Sort {
            value: value.to_string(),
        }
    }
}

impl Ord for Sort {
    fn cmp(&self, other: &Self) -> Ordering {
        compare(&self.value, &other.value)
    }
}
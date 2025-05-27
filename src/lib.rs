use std::ops::Range;
use winnow::LocatingSlice;

mod error;
mod token;

pub type RawInput<'a> = LocatingSlice<&'a str>;
pub type Span = Range<usize>;

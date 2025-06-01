#![allow(dead_code)]

use winnow::LocatingSlice;

mod error;
mod parser;
mod token;

pub(crate) type RawInput<'a> = LocatingSlice<&'a str>;

pub(crate) type Span = (usize, usize);

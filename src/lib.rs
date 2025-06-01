#![allow(dead_code)]

use winnow::LocatingSlice;

mod error;
mod lexing;
mod parser;
mod token_data;

pub(crate) type RawInput<'a> = LocatingSlice<&'a str>;

pub(crate) type Span = (usize, usize);

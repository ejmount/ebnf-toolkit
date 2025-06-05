#![allow(dead_code)]
//#![allow(warnings)]
#![warn(explicit_outlives_requirements)]
#![warn(missing_debug_implementations)]
#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]
#![warn(missing_copy_implementations)]
#![warn(redundant_lifetimes)]
//#![warn(missing_docs)]
#![warn(unreachable_pub)]
#![warn(unused_crate_dependencies)]
#![warn(unused_qualifications)]

use winnow::LocatingSlice;

mod error;
mod lexing;
mod parser;
mod token_data;

pub(crate) type RawInput<'a> = LocatingSlice<&'a str>;

pub(crate) type Span = (usize, usize);

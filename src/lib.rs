#![allow(dead_code)]

use token::Token;
use winnow::{LocatingSlice, stream::TokenSlice};

mod error;
mod parser;
mod token;

pub(crate) type RawInput<'a> = LocatingSlice<&'a str>;
pub(crate) type LexedInput<'a> = TokenSlice<'a, Token<'a>>;
pub(crate) type Span = (usize, usize);

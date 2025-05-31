#![allow(dead_code)]

use token::Token;
use winnow::{LocatingSlice, stream::TokenSlice};

mod error;
mod parser;
mod token;

pub type RawInput<'a> = LocatingSlice<&'a str>;
pub type LexedInput<'a> = TokenSlice<'a, Token<'a>>;
pub type Span = (usize, usize);

use crate::RawInput;
use winnow::error::{ContextError, ParseError};

#[derive(Debug, Clone, PartialEq)]
pub enum EbnfError<'a> {
    LexError(ParseError<RawInput<'a>, ContextError>),
}

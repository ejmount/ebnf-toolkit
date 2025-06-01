use crate::{
    RawInput,
    token_data::{LexedInput, TokenKind, TokenSet},
};
use winnow::error::{ContextError, ParseError};

#[derive(Debug, Clone, PartialEq)]
pub enum EbnfError<'a> {
    LexError(ParseError<RawInput<'a>, ContextError>),
    ParseError(ParseError<LexedInput<'a>, ContextError<TokenError>>),
}

impl<'a> From<ParseError<RawInput<'a>, ContextError>> for EbnfError<'a> {
    fn from(value: ParseError<RawInput<'a>, ContextError>) -> Self {
        EbnfError::LexError(value)
    }
}

impl<'a> From<ParseError<LexedInput<'a>, ContextError<TokenError>>> for EbnfError<'a> {
    fn from(value: ParseError<LexedInput<'a>, ContextError<TokenError>>) -> Self {
        EbnfError::ParseError(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TokenError {
    pub expected: TokenSet,
    pub found: Option<TokenKind>,
}

impl From<TokenError> for ContextError<TokenError> {
    fn from(value: TokenError) -> Self {
        let mut ctx = ContextError::new();
        ctx.push(value);
        ctx
    }
}

pub(crate) type TokenContext = ContextError<TokenError>;

use crate::{LexedInput, RawInput, token::TokenKind};
use winnow::error::{ContextError, ParseError};

#[derive(Debug, Clone, PartialEq)]
pub enum EbnfError<'a, 'err> {
    LexError(ParseError<RawInput<'a>, ContextError>),
    ParseError(ParseError<LexedInput<'a>, ContextError<TokenContext<'err>>>),
}

impl<'a, 'err> From<ParseError<RawInput<'a>, ContextError>> for EbnfError<'a, 'err> {
    fn from(value: ParseError<RawInput<'a>, ContextError>) -> Self {
        EbnfError::LexError(value)
    }
}

impl<'a, 'err> From<ParseError<LexedInput<'a>, ContextError<TokenContext<'err>>>>
    for EbnfError<'a, 'err>
{
    fn from(value: ParseError<LexedInput<'a>, ContextError<TokenContext<'err>>>) -> Self {
        EbnfError::ParseError(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenContext<'a> {
    pub expected: &'a [TokenKind],
    pub found: Option<TokenKind>,
}

impl<'a> From<TokenContext<'a>> for ContextError<TokenContext<'a>> {
    fn from(value: TokenContext<'a>) -> Self {
        let mut ctx = ContextError::new();
        ctx.push(value);
        ctx
    }
}

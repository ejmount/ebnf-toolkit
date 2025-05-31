use std::{fmt::Display, ops::Range};

use strum::{EnumDiscriminants, IntoStaticStr, VariantArray};
use winnow::{
    ModalResult, Parser,
    ascii::{alpha1, alphanumeric0, space1},
    combinator::{alt, delimited, opt, repeat},
    error::ContextError,
    stream::{ContainsToken, Location, Stream},
    token::none_of,
};

use crate::{
    LexedInput, RawInput, Span,
    error::{EbnfError, TokenContext},
};

#[derive(Debug, Clone, Copy, Eq)]
pub struct Token<'a> {
    span: Span,
    payload: TokenPayload<'a>,
}

impl Token<'_> {
    pub fn payload(&self) -> TokenPayload<'_> {
        self.payload
    }
}

impl Display for Token<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl<'a> ContainsToken<Token<'a>> for LexedInput<'a> {
    fn contains_token(&self, token: Token<'a>) -> bool {
        self.contains(&token)
    }
}

impl<'a> Location for Token<'a> {
    fn current_token_start(&self) -> usize {
        self.span.0
    }
    fn previous_token_end(&self) -> usize {
        self.span.0
    }
}

impl PartialEq for Token<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.payload == other.payload
    }
}

#[derive(EnumDiscriminants, IntoStaticStr, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[strum_discriminants(name(TokenKind), derive(IntoStaticStr, VariantArray))]
pub enum TokenPayload<'a> {
    Identifier(&'a str),
    Equals(&'a str),
    Termination,
    Alternation,
    Optional,
    String(&'a str),
    OpeningGroup,
    ClosingGroup,
    OpeningSquare,
    ClosingSquare,
    Whitespace(&'a str),
    Newline,
}

impl TokenKind {
    fn promote(self) -> &'static [Self] {
        let pos = Self::VARIANTS.iter().position(|&k| k == self).unwrap();
        &Self::VARIANTS[pos..pos + 1]
    }
}

impl<'a> Parser<LexedInput<'a>, Token<'a>, ContextError<TokenContext<'static>>> for TokenKind {
    fn parse_next(
        &mut self,
        input: &mut LexedInput<'a>,
    ) -> Result<Token<'a>, ContextError<TokenContext<'static>>> {
        TokenSetParser(self.promote()).parse_next(input)
    }
}

impl ContainsToken<&'_ Token<'_>> for &[TokenKind] {
    fn contains_token(&self, token: &'_ Token<'_>) -> bool {
        self.contains(&TokenKind::from(&token.payload))
    }
}

impl<const N: usize> ContainsToken<&'_ Token<'_>> for [TokenKind; N] {
    fn contains_token(&self, token: &'_ Token<'_>) -> bool {
        (&self[..]).contains_token(token)
    }
}

impl From<Token<'_>> for TokenKind {
    fn from(value: Token<'_>) -> Self {
        value.payload.into()
    }
}

pub(crate) struct TokenSetParser<'slice>(&'slice [TokenKind]);

impl<'input, 'slice> Parser<LexedInput<'input>, Token<'input>, ContextError<TokenContext<'slice>>>
    for TokenSetParser<'slice>
{
    fn parse_next(
        &mut self,
        input: &mut LexedInput<'input>,
    ) -> Result<Token<'input>, ContextError<TokenContext<'slice>>> {
        let TokenSetParser(token_set) = self;
        match input.next_token() {
            Some(tok) if token_set.contains(&TokenKind::from(tok.payload)) => Ok(*tok),
            Some(tok) => Err(TokenContext {
                expected: token_set,
                found: Some(tok.payload.into()),
            }),
            None => Err(TokenContext {
                expected: token_set,
                found: None,
            }),
        }
        .map_err(|e| {
            let mut ctx = ContextError::new();
            ctx.push(e);
            ctx
        })
    }
}

pub(crate) fn any_tag(slice: &[TokenKind]) -> TokenSetParser {
    TokenSetParser(slice)
}

// impl<'a> Parser<LexedInput<'a>, Token<'a>, ContextError<TokenContext>> for &[TokenKind] {
//     fn parse_next(
//         &mut self,
//         input: &mut LexedInput<'a>,
//     ) -> winnow::Result<Token<'a>, ContextError<TokenContext>> {
//         parse_token_group(self, input)
//     }
// }

// impl<'a, const N: usize> Parser<LexedInput<'a>, Token<'a>, ContextError<TokenContext>>
//     for [TokenKind; N]
// {
//     fn parse_next(
//         &mut self,
//         input: &mut LexedInput<'a>,
//     ) -> winnow::Result<Token<'a>, ContextError<TokenContext>> {
//         parse_token_group(self, input)
//     }
// }

// fn parse_token_group<'a>(
//     kinds: &[TokenKind],
//     input: &mut winnow::stream::TokenSlice<'a, Token<'a>>,
// ) -> Result<Token<'a>, ContextError<TokenContext>> {
//     if let Some(tok) = input.next_token() {
//         if kinds.contains(&TokenKind::from(tok.payload)) {
//             Ok(*tok)
//         } else {
//             let mut ctx = ContextError::new();
//             ctx.push(TokenContext {
//                 expected: kinds.to_vec(),
//                 found: Some(tok.payload.into()),
//             });
//             Err(ctx)
//         }
//     } else {
//         let mut ctx = ContextError::new();
//         ctx.push(TokenContext {
//             expected: kinds.to_vec(),
//             found: None,
//         });
//         Err(ctx)
//     }
// }

pub fn parse_identifier<'a>(input: &mut RawInput<'a>) -> ModalResult<&'a str> {
    (alpha1, alphanumeric0).take().parse_next(input)
}

pub fn parse_string<'a>(input: &mut RawInput<'a>) -> ModalResult<&'a str> {
    delimited(
        "'",
        repeat(0.., none_of(&['\''])).fold(|| (), |_, _| ()).take(),
        "'",
    )
    .parse_next(input)
}

fn parse_token<'a>(input: &mut RawInput<'a>) -> ModalResult<Token<'a>> {
    let (kind, Range { start, end }) = alt((
        parse_identifier.map(TokenPayload::Identifier),
        parse_string.map(TokenPayload::String),
        "=".map(TokenPayload::Equals),
        "::=".map(TokenPayload::Equals),
        ";".value(TokenPayload::Termination),
        "|".value(TokenPayload::Alternation),
        "?".value(TokenPayload::Optional),
        "(".value(TokenPayload::OpeningGroup),
        ")".value(TokenPayload::ClosingGroup),
        "[".value(TokenPayload::OpeningSquare),
        "]".value(TokenPayload::ClosingSquare),
        space1.map(TokenPayload::Whitespace),
        (opt("\r"), "\n").value(TokenPayload::Newline),
    ))
    .with_span()
    .parse_next(input)?;

    Ok({
        Token {
            span: (start, end),
            payload: kind,
        }
    })
}

#[allow(unused)]
pub fn tokenize<'a, 'err>(input: RawInput<'a>) -> Result<Vec<Token<'a>>, EbnfError<'a, 'err>> {
    repeat(0.., parse_token)
        .parse(input)
        .map_err(EbnfError::LexError)
}

#[cfg(test)]
mod test {
    use winnow::LocatingSlice;

    use super::tokenize;

    #[test]
    fn basic_test() {
        let input =
            "message       ::= ['@' tags SPACE] [':' source SPACE ] command [parameters] crlf;";

        let input = LocatingSlice::new(input);
        let tokens = tokenize(input);

        for i in tokens.unwrap() {
            println!("{i:?}");
        }
        panic!()
    }
}

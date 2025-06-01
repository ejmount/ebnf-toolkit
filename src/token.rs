use arrayvec::ArrayVec;
use std::{
    fmt::Debug,
    ops::{Deref, Range},
};

use strum::{Display, EnumDiscriminants, EnumProperty, IntoStaticStr, VariantArray};
use winnow::{
    ModalResult, Parser,
    ascii::{alpha1, alphanumeric0, space1},
    combinator::{alt, delimited, opt, repeat},
    error::ContextError,
    stream::{Accumulate, Stream, TokenSlice},
    token::none_of,
};

use crate::{
    RawInput, Span,
    error::{EbnfError, TokenContext, TokenError},
};

// Inherit lots of winnow machinery for the view into the tokens
pub(crate) type LexedInput<'a> = TokenSlice<'a, Token<'a>>;

#[derive(Clone, Copy, Eq)]
pub(crate) struct Token<'a> {
    pub(crate) span: Span,
    pub(crate) payload: TokenPayload<'a>,
}

impl Debug for Token<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use TokenPayload::*;
        let kind = TokenKind::from(self.payload);
        let (start, end) = self.span;
        write!(f, "{kind}[{start},{end}]")?;
        match self.payload {
            Identifier(s) | Whitespace(s) | String(s) => write!(f, "(\"{}\")", s.escape_debug())?,
            Equals | Termination | Alternation | Optional | OpeningGroup | ClosingGroup
            | OpeningSquare | ClosingSquare | Newline => {}
        };
        Ok(())
    }
}

impl PartialEq for Token<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.payload == other.payload
    }
}

#[derive(EnumDiscriminants, IntoStaticStr, EnumProperty)]
#[strum_discriminants(
    name(TokenKind),
    derive(IntoStaticStr, VariantArray, Display, PartialOrd, Ord)
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenPayload<'a> {
    Identifier(&'a str),
    Equals,
    Termination,
    Alternation,
    Optional,
    String(&'a str),
    OpeningGroup,
    ClosingGroup,
    OpeningSquare,
    ClosingSquare,
    #[strum(props(trivial = true))]
    Whitespace(&'a str),
    Newline,
}

impl<'a> Parser<LexedInput<'a>, Token<'a>, TokenContext> for TokenKind {
    fn parse_next(&mut self, input: &mut LexedInput<'a>) -> Result<Token<'a>, TokenContext> {
        TokenSet(ArrayVec::from_iter([*self])).parse_next(input)
    }
}

impl From<Token<'_>> for TokenKind {
    fn from(value: Token<'_>) -> Self {
        value.payload.into()
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub(crate) struct TokenSet(ArrayVec<TokenKind, { TokenKind::VARIANTS.len() }>);

impl TokenSet {
    pub(crate) fn new(tokens: &[TokenKind]) -> TokenSet {
        let mut set = ArrayVec::new();

        for t in tokens {
            if !set.contains(t) {
                set.push(*t);
            }
        }
        set.sort();

        TokenSet(set)
    }
}

impl<'input> Parser<LexedInput<'input>, Token<'input>, TokenContext> for TokenSet {
    fn parse_next(
        &mut self,
        input: &mut LexedInput<'input>,
    ) -> Result<Token<'input>, TokenContext> {
        let TokenSet(token_set) = self;
        match input.next_token() {
            Some(tok) if token_set.contains(&TokenKind::from(tok.payload)) => Ok(*tok),
            Some(tok) => Err(TokenError {
                expected: TokenSet::new(token_set),
                found: Some(tok.payload.into()),
            }),
            None => Err(TokenError {
                expected: TokenSet::new(token_set),
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
// Exists for readability, to resemble `one_of`
#[inline(always)]
pub(crate) fn any_token(slice: &[TokenKind]) -> TokenSet {
    TokenSet::new(slice)
}

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
        "::=".value(TokenPayload::Equals),
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

/// An owning buffer of Tokens, where most `LexedInput` is going to be pointing to.
/// This has an explicit name so there's some control over what interface it has because at some point
/// it'll need generalized for no_std
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TokenStore<'a>(Vec<Token<'a>>);
impl<'a> Deref for TokenStore<'a> {
    type Target = [Token<'a>];
    fn deref(&self) -> &Self::Target {
        &self.0[..]
    }
}

impl<'a> Accumulate<Token<'a>> for TokenStore<'a> {
    fn initial(capacity: Option<usize>) -> Self {
        TokenStore(Vec::with_capacity(capacity.unwrap_or(0)))
    }

    fn accumulate(&mut self, acc: Token<'a>) {
        self.0.push(acc);
    }
}

pub(crate) fn tokenize<'a>(input: RawInput<'a>) -> Result<TokenStore<'a>, EbnfError<'a>> {
    repeat(0.., parse_token)
        .fold(
            || TokenStore(vec![]),
            |mut store, token| {
                if token.payload.get_bool("trivial").is_none() {
                    store.0.push(token);
                }
                store
            },
        )
        .parse(input)
        .map_err(EbnfError::LexError)
}

#[cfg(test)]
mod test {
    use insta::assert_compact_debug_snapshot;
    use winnow::LocatingSlice;

    use super::tokenize;

    #[test]
    fn basic_token_test() {
        let input =
            "message       ::= ['@' tags SPACE] [':' source SPACE ] command [parameters] crlf;";

        let input = LocatingSlice::new(input);
        let tokens = tokenize(input).unwrap();

        assert_compact_debug_snapshot!(&tokens[..], @r#"[Identifier[0,7]("message"), Equals[14,17], OpeningSquare[18,19], String[19,22]("@"), Identifier[23,27]("tags"), Identifier[28,33]("SPACE"), ClosingSquare[33,34], OpeningSquare[35,36], String[36,39](":"), Identifier[40,46]("source"), Identifier[47,52]("SPACE"), ClosingSquare[53,54], Identifier[55,62]("command"), OpeningSquare[63,64], Identifier[64,74]("parameters"), ClosingSquare[74,75], Identifier[76,80]("crlf"), Termination[80,81]]"#);
    }
}

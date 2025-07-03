use std::{
    borrow::Borrow,
    fmt::{Debug, Display},
    iter::IntoIterator,
    ops::{Deref, Range},
    vec::Vec,
};

use arrayvec::ArrayVec;
use display_tree::DisplayTree;
use logos::{Lexer, Logos};
use strum::{Display, EnumDiscriminants, EnumProperty, IntoStaticStr, VariantArray};
use winnow::{
    Parser,
    error::ContextError,
    stream::{Stream, TokenSlice},
};

use crate::{
    //    container::Vec,
    container::MyVec,
    error::{InternalErrorType, TokenError},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Span {
    pub(crate) start: usize,
    pub(crate) end: usize,
}

impl From<Range<usize>> for Span {
    fn from(Range { start, end }: Range<usize>) -> Self {
        Span { start, end }
    }
}

impl Span {
    pub(crate) fn union(s: Span, t: Span) -> Span {
        Span {
            start: s.start.min(t.start),
            end: s.end.max(t.end),
        }
    }
}

impl Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Span { start, end } = self;
        write!(f, "[{start}..{end}]")
    }
}

// Inherit lots of winnow machinery for the view into the tokens
// The raw original input lives longer than the stored tokens
// The borrow checker ought to be able to merge both of these but seems to get confused by &mut Lexed showing up all the time
pub(crate) type LexedInput<'input, 'storage> = TokenSlice<'storage, Token<'input>>;

#[derive(Clone, Copy, Eq, DisplayTree)]
pub(crate) struct Token<'a> {
    pub(crate) span: Span,
    pub(crate) payload: TokenPayload<'a>,
}

impl Display for Token<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Debug for Token<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use TokenPayload::*;
        let kind = self.payload.kind();
        let span = &self.span;
        //let Span { start, end } = self.span;
        write!(f, "{kind} {span}")?;
        match &self.payload {
            Regex(s) | Identifier(s) | Whitespace(s) | String(s) => {
                write!(f, "(\"{}\")", s.escape_debug())
            }
            Kleene | Repeat | Equals | Termination | Alternation | Optional | OpeningGroup
            | ClosingGroup | OpeningSquare | ClosingSquare | Newline => Ok(()),
        }

        //Ok(())
    }
}

impl PartialEq for Token<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.payload == other.payload
    }
}

#[derive(EnumDiscriminants, IntoStaticStr, EnumProperty)]
#[strum_discriminants(name(TokenKind), derive(VariantArray, Display, PartialOrd, Ord))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Logos)]
#[logos(skip r"[[:space:]]")]
pub enum TokenPayload<'a> {
    #[regex("[A-Za-z_][A-Za-z0-9_]*")]
    Identifier(&'a str),
    #[strum(props(string = "="))]
    #[token("=")]
    #[token("::=")]
    Equals,
    #[strum(props(string = ";"))]
    #[token(";")]
    Termination,
    #[strum(props(string = "|"))]
    #[token("|")]
    Alternation,
    #[strum(props(string = "?"))]
    #[token("?")]
    Optional,
    #[strum(props(string = "*"))]
    #[token("*")]
    Kleene,
    #[strum(props(string = "+"))]
    #[token("+")]
    Repeat,
    #[regex("'[^']+'", |l| &l.slice()[1..l.slice().len()-1])]
    String(&'a str),
    #[regex("#\".+\"", |l| &l.slice()[2..l.slice().len()-1])]
    Regex(&'a str),
    #[strum(props(string = "("))]
    #[token("(")]
    OpeningGroup,
    #[strum(props(string = ")"))]
    #[token(")")]
    ClosingGroup,
    #[strum(props(string = "["))]
    #[token("[")]
    OpeningSquare,
    #[strum(props(string = "]"))]
    #[token("]")]
    ClosingSquare,
    #[strum(props(trivial = true))]
    Whitespace(&'a str),
    #[strum(props(string = "\n", trivial = true))]
    Newline,
}

impl Display for TokenPayload<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl TokenPayload<'_> {
    fn kind(&self) -> TokenKind {
        TokenKind::from(self)
    }
}

impl<'a, 'b> Parser<LexedInput<'a, 'b>, Token<'a>, ContextError<InternalErrorType<'a>>>
    for TokenKind
{
    fn parse_next(
        &mut self,
        input: &mut LexedInput<'a, 'b>,
    ) -> Result<Token<'a>, ContextError<InternalErrorType<'a>>> {
        TokenSet(ArrayVec::from_iter([*self])).parse_next(input)
    }
}

impl<'a, B: Borrow<Token<'a>>> From<B> for TokenKind {
    fn from(value: B) -> Self {
        let p: &TokenPayload = &value.borrow().payload;
        TokenKind::from(p)
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

impl<'input, 'b>
    Parser<LexedInput<'input, 'b>, Token<'input>, ContextError<InternalErrorType<'input>>>
    for TokenSet
{
    fn parse_next(
        &mut self,
        input: &mut LexedInput<'input, 'b>,
    ) -> Result<Token<'input>, ContextError<InternalErrorType<'input>>> {
        let TokenSet(token_set) = self.clone();
        let ie: Result<Token<'input>, InternalErrorType> = match input.next_token() {
            Some(tok) if token_set.contains(&tok.payload.kind()) => Ok(*tok),
            Some(tok) => Err(TokenError {
                expected: TokenSet::new(&token_set),
                found: Some(*tok),
            }
            .into()),
            None => Err(TokenError {
                expected: TokenSet::new(&token_set),
                found: None,
            }
            .into()),
        };
        Ok(ie?)
    }
}

/// An owning buffer of Tokens, where most `LexedInput` is going to be pointing to.
/// This has an explicit name so there's some control over what interface it has because at some point
/// it'll need generalized for ``no_std``
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TokenStore<'a>(pub(crate) Vec<Token<'a>>);

impl TokenStore<'_> {
    pub(crate) fn new(data: Vec<Token<'_>>) -> TokenStore<'_> {
        TokenStore(data)
    }
}

// impl<'a> TokenStore<'a> {
//     pub(crate) fn accumulator<P, I, E>(
//         repeater: winnow::combinator::Repeat<P, I, Token<'a>, (), E>,
//     ) -> impl Parser<I, TokenStore<'a>, E>
//     where
//         P: Parser<I, Token<'a>, E>,
//         I: Stream,
//         E: ParserError<I>,
//     {
//         repeater.fold(
//             || TokenStore(Vec::new()),
//             |mut store, token| {
//                 if token.payload.get_bool("trivial").is_none() {
//                     store.0.push(token);
//                 }
//                 store
//             },
//         )
//     }
// }

impl<'a> Deref for TokenStore<'a> {
    type Target = [Token<'a>];
    fn deref(&self) -> &Self::Target {
        &self.0[..]
    }
}

impl<'a> IntoIterator for TokenStore<'a> {
    type Item = Token<'a>;
    type IntoIter = <Vec<Token<'a>> as IntoIterator>::IntoIter;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, 'b> IntoIterator for &'b TokenStore<'a> {
    type Item = &'b Token<'a>;
    type IntoIter = <&'b Vec<Token<'a>> as IntoIterator>::IntoIter;
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

// // Exists for readability, to resemble `one_of`
// #[inline]
// pub(crate) fn any_token(slice: &[TokenKind]) -> TokenSet {
//     TokenSet::new(slice)
// }

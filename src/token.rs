use winnow::{
    ModalResult, Parser,
    ascii::{alpha1, alphanumeric0, space1},
    combinator::{alt, delimited, opt, repeat},
    token::none_of,
};

use crate::error::EbnfError;
use crate::{RawInput, Span};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Token<'a> {
    span: Span,
    kind: TokenType<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenType<'a> {
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
    let mut kind_parser = alt((
        parse_identifier.map(TokenType::Identifier),
        parse_string.map(TokenType::String),
        "=".map(TokenType::Equals),
        "::=".map(TokenType::Equals),
        ";".value(TokenType::Termination),
        "|".value(TokenType::Alternation),
        "?".value(TokenType::Optional),
        "(".value(TokenType::OpeningGroup),
        ")".value(TokenType::ClosingGroup),
        "[".value(TokenType::OpeningSquare),
        "]".value(TokenType::ClosingSquare),
        space1.map(TokenType::Whitespace),
        (opt("\r"), "\n").map(|_| TokenType::Newline),
    ))
    .with_span();

    let (kind, span) = kind_parser.parse_next(input)?;

    Ok(Token { span, kind })
}

#[allow(unused)]
pub fn tokenize<'a>(input: RawInput<'a>) -> Result<Vec<Token<'a>>, EbnfError<'a>> {
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
            "message       ::= ['@' tags SPACE] ~[':' source SPACE ] command [parameters] crlf;";

        let input = LocatingSlice::new(input);
        let tokens = tokenize(input);

        for i in tokens.unwrap() {
            println!("{i:?}");
        }
        panic!()
    }
}

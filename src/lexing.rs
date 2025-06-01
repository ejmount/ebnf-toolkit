use std::ops::Range;

use winnow::{
    ModalResult, Parser,
    ascii::{alpha1, alphanumeric0, space1},
    combinator::{alt, delimited, opt, repeat},
    token::none_of,
};

use crate::{
    RawInput,
    error::EbnfError,
    token_data::{Token, TokenPayload, TokenStore},
};

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

pub(crate) fn tokenize<'a>(input: RawInput<'a>) -> Result<TokenStore<'a>, EbnfError<'a>> {
    TokenStore::accumulator(repeat(0.., parse_token))
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

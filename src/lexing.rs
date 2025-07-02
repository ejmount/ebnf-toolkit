use std::ops::Range;

use strum::EnumProperty;
use winnow::{
    ModalResult, Parser,
    ascii::{alpha1, alphanumeric1, space1},
    combinator::{alt, delimited, opt, repeat},
    token::none_of,
};

use crate::{
    RawInput,
    //container::Vec,
    error::EbnfError,
    token_data::{Span, Token, TokenPayload, TokenStore},
};

pub(crate) fn parse_identifier<'a>(input: &mut RawInput<'a>) -> ModalResult<&'a str> {
    (
        alt((alpha1, "_")),
        repeat(0.., alt((alphanumeric1, "_"))).fold(|| (), |(), _| ()),
    )
        .take()
        .parse_next(input)
}

pub(crate) fn parse_string<'a>(input: &mut RawInput<'a>) -> ModalResult<&'a str> {
    delimited(
        "'",
        repeat(0.., none_of(&['\''])).fold(|| (), |(), _| ()).take(),
        "'",
    )
    .parse_next(input)
}

pub(crate) fn parse_regex<'a>(input: &mut RawInput<'a>) -> ModalResult<&'a str> {
    delimited(
        "#'",
        repeat(0.., none_of(&['\''])).fold(|| (), |(), _| ()).take(),
        "'",
    )
    .parse_next(input)
}

fn parse_token<'a>(input: &mut RawInput<'a>) -> ModalResult<Token<'a>> {
    let (payload, Range { start, end }) = alt((
        parse_identifier.map(TokenPayload::Identifier),
        parse_string.map(TokenPayload::String),
        parse_regex.map(TokenPayload::Regex),
        "=".value(TokenPayload::Equals),
        "::=".value(TokenPayload::Equals),
        ";".value(TokenPayload::Termination),
        "|".value(TokenPayload::Alternation),
        "?".value(TokenPayload::Optional),
        "*".value(TokenPayload::Kleene),
        "+".value(TokenPayload::Repeat),
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
            span: Span { start, end },
            payload,
        }
    })
}

pub(crate) fn tokenize(mut input: RawInput<'_>) -> Result<TokenStore<'_>, EbnfError<'_, '_>> {
    let mut container = Vec::default();
    let cursor = &mut input;
    while !cursor.is_empty() {
        match parse_token(cursor) {
            Ok(t) => {
                if t.payload.get_bool("trivial").is_none() {
                    container.push(t);
                }
            }
            Err(e) => return Err(EbnfError::LexError2(e, input)),
        }
    }

    Ok(TokenStore::new(container))
}

#[cfg(test)]
mod test {
    use ariadne::{Label, Source};
    use insta::assert_compact_debug_snapshot;
    use proptest::test_runner::TestRunner;
    use winnow::LocatingSlice;

    use crate::{error::BracketingError, token_data::Span};

    use super::tokenize;

    #[test]
    fn basic_token_test() {
        let input =
            "message       ::= ['@' tags SPACE] [':' source SPACE ] command [parameters] crlf;";

        let input = LocatingSlice::new(input);
        let tokens = tokenize(input).unwrap();

        assert_compact_debug_snapshot!(&tokens[..], @r#"[Identifier [0..7]("message"), Equals [14..17], OpeningSquare [18..19], String [19..22]("@"), Identifier [23..27]("tags"), Identifier [28..33]("SPACE"), ClosingSquare [33..34], OpeningSquare [35..36], String [36..39](":"), Identifier [40..46]("source"), Identifier [47..52]("SPACE"), ClosingSquare [53..54], Identifier [55..62]("command"), OpeningSquare [63..64], Identifier [64..74]("parameters"), ClosingSquare [74..75], Identifier [76..80]("crlf"), Termination [80..81]]"#);
    }

    // #[test]
    // fn bracket_success() {
    //     const SRC: &str = "([\'a\'])";
    //     let tokens = tokenize(LocatingSlice::new(SRC)).unwrap();
    //     let bracketed_tokens = brackets(tokens).unwrap();

    //     assert_compact_debug_snapshot!(&bracketed_tokens[..], @r#"[UnparsedGroup[2,5]<TokenStore([String[2,5]("a")])>]"#);
    // }

    // #[test]
    // fn bracket_dangling() {
    //     const SRC: &str = "(([\'a\'])";
    //     let tokens = tokenize(LocatingSlice::new(SRC)).unwrap();
    //     let e = brackets(tokens).unwrap_err();
    //     panic!("{e}")
    // }
}

use std::{
    fmt::{Debug, Display},
    ops::Range,
};

use logos::Logos;
use strum::{Display, EnumDiscriminants, EnumProperty, IntoStaticStr, VariantArray};

use crate::error::EbnfError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Span {
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

#[derive(Clone, Copy, Eq)]
pub(crate) struct Token<'a> {
    pub(crate) span: Span,
    pub(crate) payload: TokenPayload<'a>,
}

impl Debug for Token<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[allow(clippy::enum_glob_use)]
        use TokenPayload::*;
        let kind = TokenKind::from(self.payload);
        let span = &self.span;

        write!(f, "{kind} {span}")?;
        match &self.payload {
            Regex(s) | Identifier(s) | String(s) => {
                write!(f, "(\"{}\")", s.escape_debug())
            }
            Kleene | Repeat | Equals | Termination | Alternation | Optional | OpeningGroup
            | ClosingGroup | OpeningSquare | ClosingSquare => Ok(()),
        }
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
    #[regex(r"[\w_]*")]
    Identifier(&'a str),
    #[regex(r##"'(?:[^'\\]|\\.)*'"##, |l| &l.slice()[1..l.slice().len()-1])]
    #[regex(r##""(?:[^"\\]|\\.)*""##, |l| &l.slice()[1..l.slice().len()-1])]
    String(&'a str),
    #[regex("#\"[^\"]+\"", |l| &l.slice()[2..l.slice().len()-1])]
    #[regex(r"#'[^']+'", |l| &l.slice()[2..l.slice().len()-1])]
    Regex(&'a str),
    #[token("=")]
    #[token("::=")]
    Equals,
    #[token(";")]
    Termination,
    #[token("|")]
    Alternation,
    #[token("?")]
    Optional,
    #[token("*")]
    Kleene,
    #[token("+")]
    Repeat,
    #[token("(")]
    OpeningGroup,
    #[token(")")]
    ClosingGroup,
    #[token("[")]
    OpeningSquare,
    #[token("]")]
    ClosingSquare,
}

pub(crate) fn tokenize(input: &str) -> Result<Vec<Token>, EbnfError<'_>> {
    let lexer = TokenPayload::lexer(input).spanned();

    let mut output = Vec::new();
    for (payload, s) in lexer {
        if let Ok(payload) = payload {
            let t = Token {
                span: s.into(),
                payload,
            };

            output.push(t);
        } else {
            return Err(EbnfError::LexError {
                input,
                offset: s.start,
            });
        }
    }
    Ok(output)
}

#[cfg(test)]
mod test {
    use insta::assert_compact_debug_snapshot;

    use crate::token_data::tokenize;

    #[test]
    fn basic_token_test() {
        let input =
            "message       ::= ['@' tags SPACE] [':' source SPACE ] command [parameters] crlf;";

        let tokens = tokenize(input).unwrap();

        assert_compact_debug_snapshot!(&tokens[..], @r#"[Identifier [0..7]("message"), Equals [14..17], OpeningSquare [18..19], String [19..22]("@"), Identifier [23..27]("tags"), Identifier [28..33]("SPACE"), ClosingSquare [33..34], OpeningSquare [35..36], String [36..39](":"), Identifier [40..46]("source"), Identifier [47..52]("SPACE"), ClosingSquare [53..54], Identifier [55..62]("command"), OpeningSquare [63..64], Identifier [64..74]("parameters"), ClosingSquare [74..75], Identifier [76..80]("crlf"), Termination [80..81]]"#);
    }

    #[test]
    fn weird_identifiers() {
        let inputs = vec![
            "Charlie",
            "Hen3ry",
            "Zoë",
            "ζωή",
            "_Underbar",
            "under_pass",
            "3113",
            "3_enry",
        ];

        let tokens: Vec<_> = inputs.into_iter().map(tokenize).collect();

        assert_compact_debug_snapshot!(tokens, @r#"[Ok([Identifier [0..7]("Charlie")]), Ok([Identifier [0..6]("Hen3ry")]), Ok([Identifier [0..4]("Zoë")]), Ok([Identifier [0..6]("ζωή")]), Ok([Identifier [0..9]("_Underbar")]), Ok([Identifier [0..10]("under_pass")]), Ok([Identifier [0..4]("3113")]), Ok([Identifier [0..6]("3_enry")])]"#);
    }

    #[test]
    fn unclosed_string() {
        let input = "'Hello";

        let a: Vec<_> = tokenize(input).unwrap_or_else(|e| panic!("{e}"));

        assert_compact_debug_snapshot!(a, @r#"[String [1..8]("Hello"), String [9..16]("world")]"#);
    }

    #[test]
    fn lex_strings() {
        let input = r#" 'Hello' "world" "escaped \" character" 'another \' escape' "#;

        let a: Vec<_> = tokenize(input).unwrap_or_else(|e| panic!("{e}"));

        assert_compact_debug_snapshot!(a, @r#"[String [1..8]("Hello"), String [9..16]("world"), String [17..39]("escaped \\\" character"), String [40..59]("another \\\' escape")]"#);
    }

    #[test]
    fn lex_failure() {
        let input = " A ? ££££";

        let err = tokenize(input).unwrap_err();

        assert_compact_debug_snapshot!(err, @r#"LexError { input: " A ? ££££", offset: 5 }"#);
    }
}

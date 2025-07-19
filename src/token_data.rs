use std::{
    fmt::{Debug, Display},
    ops::Range,
};

use logos::{Lexer, Logos, Skip};
use proptest_derive::Arbitrary;
use strum::{Display, EnumDiscriminants, EnumProperty, IntoStaticStr, VariantArray};

use crate::error::EbnfError;

#[allow(unused)]
pub(crate) const DUMMY_SPAN: Span = Span {
    start: usize::MAX - 1,
    end: usize::MAX,
    line_offset_start: (u32::MAX - 1, 0),
    line_offset_end: (u32::MAX - 1, 2),
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Arbitrary)]
pub struct Span {
    start: usize,
    end: usize,
    // No single line will be 4 billion bytes long and we're unlikely to have 4 billion lines
    // And this structure is used all over the place so we do save something significant if it's smaller
    line_offset_start: (u32, u32),
    line_offset_end: (u32, u32),
}

impl Span {
    pub fn start(&self) -> usize {
        self.start
    }
    pub fn end(&self) -> usize {
        self.end
    }
    pub fn range(&self) -> Range<usize> {
        self.start..self.end
    }

    pub(crate) fn union<'a>(iter: impl Iterator<Item = &'a Node<'a>>) -> Span {
        iter.map(Node::span)
            .reduce(|s, t| {
                let min = if s.start < t.start { s } else { t };
                let max = if s.end > t.end { s } else { t };

                Span {
                    start: min.start,
                    end: max.end,
                    line_offset_start: min.line_offset_start,
                    line_offset_end: max.line_offset_end,
                }
            })
            .expect("Asked for span of empty list")
    }
}
    }
}

impl Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Span {
            line_offset_start: (start_line, start_off),
            line_offset_end: (end_line, end_off),
            ..
        } = self;
        write!(f, "[{start_line}:{start_off}..{end_line}:{end_off}]",)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
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
            | ClosingGroup | OpeningSquare | ClosingSquare | OpeningBrace | ClosingBrace
            | Newline => Ok(()),
        }
    }
}

#[derive(EnumDiscriminants, IntoStaticStr, EnumProperty)]
#[strum_discriminants(name(TokenKind), derive(VariantArray, Display, PartialOrd, Ord))]
#[derive(Logos, Debug, Clone, Copy, PartialEq, Eq)]
#[logos(skip "[[:space:]]")]
#[logos(skip ",")]
#[logos(skip "// [^\\n\\r]*")]
#[logos(extras = (usize, usize))]
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
    #[token("/")]
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
    #[token("{")]
    OpeningBrace,
    #[token("}")]
    ClosingBrace,
    #[token("\n", line_counter, priority = 20)]
    #[token("\r", line_counter, priority = 20)]
    #[token("\r\n", line_counter)]
    Newline,
}

fn line_counter<'a>(lex: &mut Lexer<'a, TokenPayload<'a>>) -> Skip {
    #[allow(clippy::naive_bytecount)]
    let lines = lex
        .slice()
        .as_bytes()
        .iter()
        .filter(|b| **b == b'\n')
        .count();
    lex.extras.0 += lines;
    lex.extras.1 = lex.span().end;
    Skip
}

pub(crate) fn tokenize(input: &str) -> Result<Vec<Token>, EbnfError<'_>> {
    let mut lexer = TokenPayload::lexer(input).spanned();

    let mut output = Vec::new();

    while let Some((payload, s)) = lexer.next() {
        let (line_count, last_newline_offset) = lexer.extras;
        if let Ok(payload) = payload {
            let Range { start, end } = s;
            let line_offset_start = start - last_newline_offset;
            let line_offset_end = end - last_newline_offset;
            #[allow(
                clippy::cast_possible_truncation,
                reason = "No line will be 2^32 bytes long"
            )]
            let span = Span {
                start,
                end,
                line_offset_start: (1 + line_count as u32, line_offset_start as u32),
                line_offset_end: (1 + line_count as u32, line_offset_end as u32),
            };

            output.push(Token { span, payload });
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

        assert_compact_debug_snapshot!(&tokens[..], @r#"[Identifier [1:0..1:7]("message"), Equals [1:14..1:17], OpeningSquare [1:18..1:19], String [1:19..1:22]("@"), Identifier [1:23..1:27]("tags"), Identifier [1:28..1:33]("SPACE"), ClosingSquare [1:33..1:34], OpeningSquare [1:35..1:36], String [1:36..1:39](":"), Identifier [1:40..1:46]("source"), Identifier [1:47..1:52]("SPACE"), ClosingSquare [1:53..1:54], Identifier [1:55..1:62]("command"), OpeningSquare [1:63..1:64], Identifier [1:64..1:74]("parameters"), ClosingSquare [1:74..1:75], Identifier [1:76..1:80]("crlf"), Termination [1:80..1:81]]"#);
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

        assert_compact_debug_snapshot!(tokens, @r#"[Ok([Identifier [1:0..1:7]("Charlie")]), Ok([Identifier [1:0..1:6]("Hen3ry")]), Ok([Identifier [1:0..1:4]("Zoë")]), Ok([Identifier [1:0..1:6]("ζωή")]), Ok([Identifier [1:0..1:9]("_Underbar")]), Ok([Identifier [1:0..1:10]("under_pass")]), Ok([Identifier [1:0..1:4]("3113")]), Ok([Identifier [1:0..1:6]("3_enry")])]"#);
    }

    #[test]
    fn lex_escaped_strings() {
        let input = r#" 'Hello' "world" "escaped \" character" 'another \' escape' "#;

        let a: Vec<_> = tokenize(input).unwrap_or_else(|e| panic!("{e}"));

        assert_compact_debug_snapshot!(a, @r#"[String [1:1..1:8]("Hello"), String [1:9..1:16]("world"), String [1:17..1:39]("escaped \\\" character"), String [1:40..1:59]("another \\\' escape")]"#);
    }

    #[test]
    fn lex_failure() {
        let input = " A ? ££££";

        let err = tokenize(input).unwrap_err();

        assert_eq!(err, err);
        assert_compact_debug_snapshot!(err, @r#"LexError { input: " A ? ££££", offset: 5 }"#);
    }
}

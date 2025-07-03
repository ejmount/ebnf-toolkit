use std::{error::Error, fmt::Display};

use crate::{
    RawInput,
    nodes::{Node, NodeKind},
    parser::LrStack,
    token_data::{LexedInput, Span, Token, TokenKind, TokenSet, TokenStore},
};
use ariadne::{Label, Source};
use winnow::error::{ContextError, ErrMode, ParseError};

#[derive(Debug, Clone, PartialEq)]
pub enum EbnfError<'a, 'b> {
    LexError(ParseError<RawInput<'a>, ContextError>),
    LexError2(ErrMode<ContextError>, RawInput<'a>),
    LexError3(&'a str),
    ParseError(ParseError<LexedInput<'a, 'b>, ContextError<TokenError<'a>>>),
    //MalformedInput(SyntaxError),
    Test1(ContextError<TokenError<'a>>),
    //Test2(ContextError<SyntaxError>),
}

// impl<'a> From<ParseError<RawInput<'a>, ContextError>> for EbnfError<'a, '_> {
//     fn from(value: ParseError<RawInput<'a>, ContextError>) -> Self {
//         EbnfError::LexError(value)
//     }
// }

impl<'a, 'b> From<ParseError<LexedInput<'a, 'b>, ContextError<TokenError<'a>>>>
    for EbnfError<'a, 'b>
{
    fn from(value: ParseError<LexedInput<'a, 'b>, ContextError<TokenError<'a>>>) -> Self {
        EbnfError::ParseError(value)
    }
}

// impl<'a> From<ContextError<TokenError<'a>>> for EbnfError<'_, '_> {
//     fn from(value: ContextError<TokenError<'a>>) -> Self {
//         todo!()
//     }
// }

// impl<'a> From<InternalErrorType<'a>> for EbnfError<'_, '_> {
//     fn from(value: InternalErrorType<'a>) -> Self {
//         todo!()
//     }
// }

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TokenError<'a> {
    pub expected: TokenSet,
    pub found: Option<Token<'a>>,
}

impl<'a> From<TokenError<'a>> for ContextError<TokenError<'a>> {
    fn from(value: TokenError<'a>) -> Self {
        let mut ctx = ContextError::new();
        ctx.push(value);
        ctx
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub(crate) struct InternalError<'a> {
    input: &'a str,
    kind: InternalErrorType<'a>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub(crate) enum InternalErrorType<'a> {
    TokenError(TokenError<'a>),
    PostfixError(PostfixError<'a>),
    UnparsedToken(Span),
    BracketingError(Span, BracketingError),
    UnexpectedSemicolon(Span),
    UnexpectedNode {
        expected: Vec<NodeKind>,
        found: Option<Node<'a>>,
    },
    IncompleteParse(LrStack<'a>, TokenStore<'a>),
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum BracketingError {
    DanglingOpen(TokenKind),
    UnexpectedClose,
    TypeMismatch {
        found: TokenKind,
        expected: TokenKind,
    },
}

impl Display for InternalError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            InternalErrorType::BracketingError(Span { start, end }, be) => {
                //let input = format!("{}", self.input);
                let mut output = vec![];
                let mut cg = ariadne::ColorGenerator::new();
                let c = cg.next();

                let text = match be {
                    BracketingError::TypeMismatch { found, expected } => {
                        format!("Found: {found}, expected: {expected}")
                    }
                    BracketingError::DanglingOpen(_t) => "Bracket never closed".to_string(),
                    _ => todo!(),
                };

                let s = Source::from(self.input.to_string());

                let report =
                    ariadne::Report::build(ariadne::ReportKind::Error, 0..self.input.len())
                        .with_message("Mismatched bracket")
                        .with_label(Label::new(start..end).with_color(c).with_message(text))
                        .finish();

                report.write_for_stdout(s, &mut output).unwrap();

                write!(f, "{}", String::from_utf8(output).unwrap())?;
            }
            _ => unreachable!(),
        }
        Ok(())
    }
}

impl Error for InternalError<'_> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }

    fn description(&self) -> &'static str {
        "description() is deprecated; use Display"
    }

    fn cause(&self) -> Option<&dyn Error> {
        self.source()
    }
}

impl<'a> From<PostfixError<'a>> for InternalErrorType<'a> {
    fn from(value: PostfixError<'a>) -> Self {
        InternalErrorType::PostfixError(value)
    }
}

impl<'a> From<TokenError<'a>> for InternalErrorType<'a> {
    fn from(value: TokenError<'a>) -> Self {
        InternalErrorType::TokenError(value)
    }
}

impl<'a> From<InternalErrorType<'a>> for ContextError<InternalErrorType<'a>> {
    fn from(value: InternalErrorType<'a>) -> Self {
        let mut ctx = ContextError::new();
        ctx.push(value);
        ctx
    }
}

// pub(crate) type TokenContext = ContextError<TokenError<'a>>;
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PostfixError<'a>(pub Token<'a>);

// match bracketed_tokens {
//     Ok(brackets) => ,
//     Err(crate::error::InternalError::BracketingError(Span { start, end }, b)) => {
//         let mut cg = ariadne::ColorGenerator::new();
//         let c = cg.next();

//         let text = match b {
//             BracketingError::TypeMismatch { found, expected } => {
//                 format!("Found: {found}, expected: {expected}")
//             }
//             _ => todo!(),
//         };

//         let report = ariadne::Report::build(ariadne::ReportKind::Error, 0..SRC.len())
//             .with_message("Mismatched bracket")
//             .with_label(Label::new(start..end).with_color(c).with_message(text))
//             .finish();

//         report.eprint(Source::from(SRC));

//         panic!();
//     }
//     e => unreachable!("{e:?}"),
// }

// pub(crate) enum InternalError<'a> {
//     O(PhantomData<&'a ()>),
// }

// impl From<SyntaxError<'_>> for InternalError<'a> {
//     fn from(value: SyntaxError<'_>) -> Self {
//         todo!()
//     }
// }

// impl From<TokenError<'a>> for InternalError<'a> {
//     fn from(value: TokenError<'a>) -> Self {
//         todo!()
//     }
// }

// impl FromExternalError<LexedInput<'_>, SyntaxError<'_>> for InternalError<'a> {
//     fn from_external_error(input: &LexedInput<'_>, e: SyntaxError<'_>) -> Self {
//         todo!()
//     }
// }

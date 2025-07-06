use std::fmt::Display;

use crate::parser::LrStack;

#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum EbnfError<'a> {
    LexError {
        input: &'a str,
        offset: usize,
    },
    ParseError {
        input: &'a str,
        offset: usize,
        /// This is a guess, it may be wrong.
        /// Its also unstable and errors in the same circumstances may produce different values for this field without notice
        reason: Option<FailureReason<'a>>,
    },
    EmptyInput,
}

impl EbnfError<'_> {
    pub fn input(&self) -> Option<&str> {
        match self {
            EbnfError::LexError { input, .. } | EbnfError::ParseError { input, .. } => Some(input),
            _ => None,
        }
    }
    pub fn offset(&self) -> Option<usize> {
        match self {
            EbnfError::LexError { offset, .. } | EbnfError::ParseError { offset, .. } => {
                Some(*offset)
            }
            _ => None,
        }
    }
}

impl PartialEq for EbnfError<'_> {
    fn eq(&self, other: &Self) -> bool {
        #[allow(clippy::enum_glob_use)]
        use EbnfError::*;
        match (self, other) {
            (EmptyInput, EmptyInput) => true,
            (this @ LexError { .. }, other @ LexError { .. })
            | (this @ ParseError { .. }, other @ ParseError { .. }) => this
                .input()
                .cmp(&other.input())
                .then(this.offset().cmp(&other.offset()))
                .is_eq(),

            _ => false,
        }
    }
}

impl Eq for EbnfError<'_> {}

#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub enum FailureReason<'a> {
    TerminatorNotEndingRule(&'a str),
    ExhaustedInput,
}

impl EbnfError<'_> {
    pub(crate) fn from_parse_error<'a>(
        input: &'a str,
        _stack: LrStack,
        offset: usize,
        reason: Option<FailureReason<'a>>,
    ) -> EbnfError<'a> {
        EbnfError::ParseError {
            input,
            offset,
            reason,
        }
    }
}

impl Display for EbnfError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ariadne::{ColorGenerator, Label, Report, ReportKind, Source};
        let input = match self {
            EbnfError::LexError { input, .. } | EbnfError::ParseError { input, .. } => *input,
            EbnfError::EmptyInput => return write!(f, "Input string was empty"),
        };

        let s = Source::from(input);

        let mut colors = ColorGenerator::new();

        let col = colors.next();

        let mut report = Report::build(ReportKind::Error, ("", 0..input.len()));

        #[allow(clippy::range_plus_one)]
        match self {
            &EbnfError::LexError { offset, .. } => {
                report = report.with_message("Tokenization error").with_label(
                    Label::new(("", offset..1 + offset))
                        .with_message("This was not recognised as the start of a valid token")
                        .with_color(col),
                );
                if input.as_bytes()[offset] == b'\'' || input.as_bytes()[offset] == b'"' {
                    report = report.with_note("Did you forget to close a string?");
                }
            }
            _ => todo!(),
        }

        let r = report.finish();

        let mut output = vec![];

        r.write(("", s), &mut output).unwrap();
        write!(f, "{}", String::from_utf8(output).unwrap())?;

        Ok(())
    }
}

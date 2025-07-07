use std::fmt::Display;

use crate::parser::LrStack;

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum EbnfError<'a> {
    LexError {
        input: &'a str,
        offset: usize,
    },
    ParseError {
        input: &'a str,
        start: usize,
        end: Option<usize>,
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
            EbnfError::LexError { offset, .. } | EbnfError::ParseError { start: offset, .. } => {
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
#[derive(Debug, Clone)]
pub enum FailureReason<'a> {
    TerminatorNotEndingRule(&'a str, LrStack<'a>),
    InvalidRuleStart,
    ExhaustedInput,
}

impl EbnfError<'_> {
    pub(crate) fn from_parse_error<'a>(
        input: &'a str,
        _stack: LrStack,
        start: usize,
        end: Option<usize>,
        reason: Option<FailureReason<'a>>,
    ) -> EbnfError<'a> {
        EbnfError::ParseError {
            input,
            start,
            end,
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
            EbnfError::EmptyInput => {
                report = report.with_message("Input was empty");
            }
            EbnfError::ParseError {
                input,
                start,
                end,
                reason,
            } => {
                report = report.with_message("Parse error").with_label(
                    Label::new(("", *start..end.unwrap_or(start + 1)))
                        .with_message(format!(
                            "Reason: {}",
                            match reason.as_ref().unwrap() {
                                FailureReason::TerminatorNotEndingRule(s, t) => format!("{t:?}"),
                                FailureReason::ExhaustedInput => "exhausted".to_owned(),
                                FailureReason::InvalidRuleStart =>
                                    "Tried to start parsing a rule here".to_owned(),
                            }
                        ))
                        .with_color(col),
                );
            }
        }

        let r = report.finish();

        let mut output = vec![];

        r.write(("", s), &mut output).unwrap();
        write!(f, "{}", String::from_utf8(output).unwrap())?;

        Ok(())
    }
}

use std::fmt::Display;

use crate::parser::LrStack;

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum EbnfError<'a> {
    LexError { input: &'a str, offset: usize },
    ParseError { input: &'a str, offset: usize },
    EmptyInput,
    //UnknownError(Vec<Node<'a>>),
}

impl EbnfError<'_> {
    pub(crate) fn from_parse_error<'a>(input: &'a str, stack: LrStack) -> EbnfError<'a> {
        dbg!(input, stack);
        todo!()
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

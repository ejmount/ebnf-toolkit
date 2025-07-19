use std::fmt::Display;

use display_tree::Style;

use crate::{
    Node,
    debug::print_vec_tree,
    nodes::{NodeKind, Operator},
    parser::LrStack,
};

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
    TerminatorNotEndingRule(Vec<Node<'a>>),
    InvalidRuleStart(Node<'a>),
    ExhaustedInput(Vec<Node<'a>>),
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

        let mut report = Report::build(ReportKind::Error, ("<input>", 0..input.len()));

        #[allow(clippy::range_plus_one)]
        match self {
            &EbnfError::LexError { offset, .. } => {
                report = report.with_message("Tokenization error").with_label(
                    Label::new(("<input>", offset..1 + offset))
                        .with_message("This was not recognised as the start of a valid token")
                        .with_color(col),
                );
                if input.as_bytes()[offset] == b'\'' || input.as_bytes()[offset] == b'"' {
                    report = report.with_note("Is this the beginning of an unclosed string?");
                }
            }
            EbnfError::EmptyInput => {
                report = report.with_message("Input was empty");
            }
            EbnfError::ParseError {
                input: _,
                start,
                end,
                reason,
            } => {
                match reason.as_ref().unwrap() {
                    FailureReason::ExhaustedInput(nodes) => {
                        report = report.with_message(format!(
                            "Parse error: Unexpected end of input at index {start}",
                        ));
                        report = print_stack(report, nodes);
                    }

                    FailureReason::InvalidRuleStart(incorrect_node) => {
                        let found_msg = match NodeKind::from(incorrect_node) {
                            NodeKind::Terminal => "string literal",
                            NodeKind::Regex => "regular expression",
                            NodeKind::UnparsedOperator => "operator",
                            NodeKind::Group => "list of terms", // unreachable?
                            NodeKind::Choice => "alternatives",
                            NodeKind::Repeated => "repetition",
                            NodeKind::Optional => "optional",
                            NodeKind::Rule | NodeKind::Nonterminal => unreachable!(),
                        };
                        report = report.with_label(    Label::new(("<input>", *start..end.unwrap_or(start + 1)))
                                .with_message(format!("Tried to start parsing a rule here; expected single non-terminal, found {found_msg}"))
                                .with_color(col),
                        );
                    }
                    FailureReason::TerminatorNotEndingRule(nodes) => {
                        for n in nodes {
                            if let Node::UnparsedOperator { span, op } = n {
                                if *op != Operator::Equals && *op != Operator::Terminator {
                                    let message = match *op {
                                        Operator::OpenedGroup | Operator::OpenedSquare => {
                                            "Possible unclosed bracket"
                                        }
                                        Operator::Kleene
                                        | Operator::Optional
                                        | Operator::Repeat => "Could not apply to preceding term",
                                        _ => "Operator not understood",
                                    };
                                    report = report.with_label(
                                        Label::new(("<input>", span.start()..span.end()))
                                            .with_color(colors.next())
                                            .with_message(message),
                                    );
                                }
                            }
                        }
                        report = report.with_label(
                            Label::new(("<input>", *start..end.unwrap_or(start + 1)))
                                .with_message(
                                    "Rule ending here did not parse successfully".to_string(),
                                )
                                .with_color(col),
                        );
                        report = print_stack(report, nodes);
                    }
                }
            }
        }

        let r = report.finish();

        let mut output = vec![];

        r.write(("<input>", s), &mut output).unwrap();
        write!(f, "{}", String::from_utf8(output).unwrap())?;

        Ok(())
    }
}

fn print_stack<'a>(
    report: ariadne::ReportBuilder<'a, (&'static str, std::ops::Range<usize>)>,
    nodes: &[Node<'_>],
) -> ariadne::ReportBuilder<'a, (&'static str, std::ops::Range<usize>)> {
    let mut nodes: Vec<_> = nodes.to_vec();
    nodes.reverse();
    let mut tree_output = String::new();
    print_vec_tree(&mut tree_output, Style::default(), &nodes).unwrap();
    report.with_note(format!(
        "The parse stack looked like this (most recent on top):\n{tree_output}",
    ))
}

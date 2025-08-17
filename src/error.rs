#![allow(clippy::range_plus_one)]

use std::{fmt::Display, ops::Range};

use ariadne::{ColorGenerator, Label, ReportBuilder};
use display_tree::Style;

use crate::{
    Expr, Span,
    debug::print_vec_tree,
    expr::{ExprKind, Operator},
    parser::LrStack,
    token_data::{Token, TokenPayload},
};

type ReportType<'a> = ReportBuilder<'a, (&'static str, Range<usize>)>;

#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
pub enum FailureReason<'a> {
    TerminatorNotEndingRule(Vec<Expr<'a>>),
    ExhaustedInput(Vec<Expr<'a>>),
}

impl Display for EbnfError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ariadne::{ColorGenerator, Label, Report, ReportKind, Source};
        let input = match self {
            EbnfError::LexError { input, .. } | EbnfError::ParseError { input, .. } => *input,
            EbnfError::EmptyInput => return write!(f, "Input string was empty"),
        };

        let s = Source::from(input);

        let mut report = Report::build(ReportKind::Error, ("<input>", 0..input.len()));

        match self {
            &EbnfError::LexError { offset, .. } => {
                let mut colors = ColorGenerator::new();

                let col = colors.next();
                report = report.with_message("Tokenization error").with_label(
                    Label::new(("<input>", offset..offset + 1))
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
                offset: start,
                reason,
            } => {
                report = handle_parse_error(report, *start, reason.as_ref());
            }
        }

        let r = report.finish();

        let mut output = vec![];

        r.write(("<input>", s), &mut output).unwrap();
        write!(f, "{}", String::from_utf8(output).unwrap())?;

        Ok(())
    }
}

fn handle_parse_error<'a>(
    mut report: ReportType<'a>,
    offset: usize,
    reason: Option<&FailureReason<'_>>,
) -> ReportType<'a> {
    let mut colors = ColorGenerator::new();

    let col = colors.next();

    let nodes = match reason.as_ref().unwrap() {
        FailureReason::ExhaustedInput(nodes) | FailureReason::TerminatorNotEndingRule(nodes) => {
            nodes
        }
    };

    for n in nodes {
        if let Expr::UnparsedOperator { span, op } = n {
            if *op != Operator::Equals && *op != Operator::Terminator {
                let message = match *op {
                    Operator::OpenedGroup | Operator::OpenedSquare => "Possible unclosed bracket",
                    Operator::Kleene | Operator::Optional | Operator::Repeat => {
                        "Could not apply to preceding term"
                    }
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

    match reason.as_ref().unwrap() {
        FailureReason::ExhaustedInput(nodes) => {
            if check_missing_terminator(nodes) {
                report = report.with_label(
                    Label::new(("<input>", offset..offset))
                        .with_message("Missing semicolon here")
                        .with_color(colors.next()),
                );
            } else {
                report = report.with_label(
                    Label::new(("<input>", offset..offset))
                        .with_message(format!("Unexpected end of input at index {offset}"))
                        .with_color(colors.next()),
                );
            }
            attach_stack_to_report(report, nodes)
        }

        FailureReason::TerminatorNotEndingRule(nodes) => {
            if let Some(equals) = nodes.iter().position(|n| {
                matches!(
                    n,
                    Expr::UnparsedOperator {
                        op: Operator::Equals,
                        ..
                    }
                )
            }) && let Some(not_identifier) = nodes.get(equals - 1)
                && ExprKind::from(not_identifier) != ExprKind::Nonterminal
            {
                let Range { start, end } = not_identifier.span().range();
                report = report.with_label(
                    Label::new(("<input>", start..end))
                        .with_message(format!(
                            "Expected identifier, found {:?}",
                            ExprKind::from(not_identifier)
                        ))
                        .with_color(colors.next()),
                );
            }
            report = report.with_label(
                Label::new(("<input>", offset..(offset + 1)))
                    .with_message("Rule ending here did not parse successfully".to_string())
                    .with_color(col),
            );
            attach_stack_to_report(report, nodes)
        }
    }
}

fn check_missing_terminator(nodes: &[Expr<'_>]) -> bool {
    let mut stack = LrStack::new();
    for n in nodes {
        stack.push_node(n.clone());
    }
    let span = Span::union(nodes.iter());
    stack.push_token(Token {
        span,
        payload: TokenPayload::Termination,
    });
    stack.reduce_until_shift_needed();
    matches!(stack.pop_node(), Some(Expr::Rule { .. }))
}

fn attach_stack_to_report<'a>(report: ReportType<'a>, nodes: &[Expr<'_>]) -> ReportType<'a> {
    let mut nodes = nodes.to_vec();
    nodes.reverse();
    let mut tree_output = String::new();
    print_vec_tree(&mut tree_output, Style::default(), &nodes).unwrap();
    report.with_note(format!(
        "The parse stack looked like this (most recent on top):\n{tree_output}",
    ))
}

use crate::{
    EbnfError, FailureReason, Rule,
    parser::LrStack,
    simplification::simplify_node,
    token_data::{Span, tokenize},
};
use std::fmt::Display;
use strum::{EnumDiscriminants, EnumProperty, IntoStaticStr, VariantNames};

#[derive(Debug, Clone, PartialEq, Eq, EnumDiscriminants)]
#[strum_discriminants(name(ExprKind), derive(VariantNames, IntoStaticStr))]
pub enum Expr<'a> {
    Literal {
        span: Span,
        str: &'a str,
    },
    Nonterminal {
        span: Span,
        name: &'a str,
    },
    Choice {
        span: Span,
        body: Vec<Expr<'a>>,
    },
    Optional {
        span: Span,
        body: Vec<Expr<'a>>,
    },
    Repetition {
        span: Span,
        body: Vec<Expr<'a>>,
        one_needed: bool,
    },
    Regex {
        span: Span,
        pattern: &'a str,
    },
    Group {
        span: Span,
        body: Vec<Expr<'a>>,
    },
    UnparsedOperator {
        span: Span,
        op: Operator,
    },
    Rule {
        span: Span,
        rule: Rule<'a>,
    },
}

impl<'a> Expr<'a> {
    pub fn new(input: &'a str) -> Result<Self, EbnfError<'a>> {
        let tokens = tokenize(input)?;
        let mut stack = LrStack::new();
        for token in tokens {
            stack.push_token(token);
            stack.reduce_until_shift_needed();
        }
        let token_stack = stack.into_parse_stack();
        if token_stack.len() == 1 {
            let mut expr = token_stack
                .into_iter()
                .next()
                .unwrap_or_else(|| unreachable!());
            simplify_node(&mut expr);
            Ok(expr)
        } else {
            Err(EbnfError::ParseError {
                input,
                offset: input.len(),
                reason: Some(FailureReason::ExhaustedInput(token_stack)),
            })
        }
    }

    pub fn span(&self) -> Span {
        match self {
            Expr::Literal { span, .. }
            | Expr::Nonterminal { span, .. }
            | Expr::Choice { span, .. }
            | Expr::Optional { span, .. }
            | Expr::Repetition { span, .. }
            | Expr::Regex { span, .. }
            | Expr::Group { span, .. }
            | Expr::UnparsedOperator { span, .. }
            | Expr::Rule { span, .. } => *span,
        }
    }

    pub(crate) fn node_pattern_code(&self) -> &'static str {
        if let Expr::UnparsedOperator { op, .. } = self {
            op.get_str("repr").unwrap()
        } else {
            let name: &str = ExprKind::from(self).into();
            &name[..1]
        }
    }

    pub(crate) fn apply_replacement(
        &mut self,
        func: &mut impl for<'b> FnMut(&Expr<'a>) -> Option<Expr<'a>>,
    ) {
        match self {
            Expr::Rule {
                span,
                rule: Rule { body, .. },
            }
            | Expr::Choice { span, body }
            | Expr::Optional { span, body }
            | Expr::Repetition { span, body, .. }
            | Expr::Group { span, body } => {
                for n in body.iter_mut() {
                    if let Some(new) = func(n) {
                        *n = new;
                    }
                    n.apply_replacement(func);
                }
                *span = Span::union(body.iter());
            }

            Expr::Regex { .. }
            | Expr::UnparsedOperator { .. }
            | Expr::Literal { .. }
            | Expr::Nonterminal { .. } => { /* no children, do nothing */ }
        }
        let res = func(self);
        if let Some(res) = res {
            *self = res;
        }
    }
}

fn write_slice(
    f: &mut std::fmt::Formatter<'_>,
    slice: &[Expr<'_>],
    sep: &'static str,
) -> std::fmt::Result {
    write!(f, "(")?;
    for (ind, child) in slice.iter().enumerate() {
        if ind > 0 {
            write!(f, "{sep}")?;
        }
        write!(f, "({child})")?;
    }
    write!(f, ")")
}

impl Display for Expr<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Nonterminal { name: str, .. } => write!(f, "{str}")?,
            Expr::Literal { str, .. } => write!(f, "\"{str}\"")?,

            Expr::Repetition {
                body, one_needed, ..
            } => {
                if *one_needed {
                    write!(f, "{{")?;
                    write_slice(f, body, " ")?;
                    write!(f, "}}")?;
                } else {
                    write!(f, "(")?;
                    write_slice(f, body, " ")?;
                    write!(f, ")*")?;
                }
            }
            Expr::Optional { body, .. } => {
                write!(f, "[")?;
                write_slice(f, body, " ")?;
                write!(f, "]")?;
            }

            Expr::Regex { pattern, .. } => write!(f, "#'{pattern}'")?,
            Expr::Group { body, .. } => write_slice(f, body, " ")?,
            Expr::Choice { body, .. } => {
                write_slice(f, body, "|")?;
            }
            Expr::UnparsedOperator { op, .. } => write!(f, "{}", op.get_str("repr").unwrap())?,
            Expr::Rule {
                rule: Rule { name, body },
                ..
            } => {
                write!(f, "{name} =")?;
                for child in body {
                    write!(f, "{child}")?;
                }
                write!(f, ";")?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumProperty, IntoStaticStr)]
pub enum Operator {
    #[strum(props(repr = "("))]
    OpenedGroup,
    #[strum(props(repr = ")"))]
    ClosedGroup,
    #[strum(props(repr = "["))]
    OpenedSquare,
    #[strum(props(repr = "]"))]
    ClosedSquare,
    #[strum(props(repr = "{"))]
    OpenedBrace,
    #[strum(props(repr = "}"))]
    ClosedBrace,
    #[strum(props(repr = ";"))]
    Terminator,
    #[strum(props(repr = "="))]
    Equals,
    #[strum(props(repr = "|"))]
    Alternation,
    #[strum(props(repr = "*"))]
    Kleene,
    #[strum(props(repr = "?"))]
    Optional,
    #[strum(props(repr = "+"))]
    Repeat,
}

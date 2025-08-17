use crate::{
    EbnfError, FailureReason, Rule,
    parser::LrStack,
    simplification::simplify_node,
    token_data::{Span, tokenize},
};
use std::fmt::Display;
use strum::{EnumDiscriminants, EnumProperty, IntoStaticStr, VariantNames};

/// A node in the syntax tree of a EBNF rule
#[derive(Debug, Clone, PartialEq, Eq, Hash, EnumDiscriminants)]
#[strum_discriminants(name(ExprKind), derive(VariantNames, IntoStaticStr))]
#[non_exhaustive]
pub enum Expr<'a> {
    /// A terminal, a string, a whole string and nothing but the string
    Literal {
        #[expect(missing_docs, reason = "Obvious")]
        span: Span,
        #[expect(missing_docs, reason = "Obvious")]
        str: &'a str,
    },
    /// the name of some other production rule
    Nonterminal {
        #[expect(missing_docs, reason = "Obvious")]
        span: Span,
        #[expect(missing_docs, reason = "Obvious")]
        name: &'a str,
    },
    /// Exactly one of the child nodes
    Choice {
        #[expect(missing_docs, reason = "Obvious")]
        span: Span,
        #[expect(missing_docs, reason = "Obvious")]
        body: Vec<Expr<'a>>,
    },
    /// Either the entire sequence of child nodes in order, or nothing
    Optional {
        #[expect(missing_docs, reason = "Obvious")]
        span: Span,
        #[expect(missing_docs, reason = "Obvious")]
        body: Vec<Expr<'a>>,
    },
    /// The child nodes, in sequence, repeated any number of times, possibly including zero.
    Repetition {
        #[expect(missing_docs, reason = "Obvious")]
        span: Span,
        #[expect(missing_docs, reason = "Obvious")]
        body: Vec<Expr<'a>>,
        /// If at least one repetition is needed or none
        one_needed: bool,
    },
    /// A regular expression on the input string.
    ///
    /// A valid regex is defined by the [regex](https://docs.rs/regex/latest/regex/) crate, over terminal characters, not over other EBNF rules.
    Regex {
        #[expect(missing_docs, reason = "Obvious")]
        span: Span,
        #[expect(missing_docs, reason = "Obvious")]
        pattern: &'a str,
    },
    /// The child nodes, in order, exactly once. You are unlikely to see this in parsing output; see [the root docstring](`crate`) for why.
    Group {
        #[expect(missing_docs, reason = "Obvious")]
        span: Span,
        #[expect(missing_docs, reason = "Obvious")]
        body: Vec<Expr<'a>>,
    },
    #[doc(hidden)]
    UnparsedOperator { span: Span, op: Operator },
    /// An entire EBNF rule. This is not the same type as [`Rule`] and user code should use the latter, but it's included as part of `Expr` for internal reasons.
    Rule {
        #[expect(missing_docs, reason = "Obvious")]
        span: Span,
        #[expect(missing_docs, reason = "Obvious")]
        rule: Rule<'a>,
    },
}

impl<'a> Expr<'a> {
    /// Parse a given string into an `Expr`. For parsing an entire rule, instead prefer [`Rule::new`].
    ///
    /// # Errors
    /// If the input string is ill-formed, an [`EbnfError`] is returned. See that type for possible reasons why.
    pub fn new(input: &'a str) -> Result<Self, EbnfError<'a>> {
        let tokens = tokenize(input)?;
        let mut stack = LrStack::new();
        for token in tokens {
            stack.push_token(token);
            stack.reduce_until_shift_needed();
        }
        let token_stack = stack.into_parse_stack();
        if token_stack.len() == 1
            && token_stack.first().map(ExprKind::from) != Some(ExprKind::UnparsedOperator)
        {
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

    /// The [`Span`] of the input this node and all of its children represent
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumProperty, IntoStaticStr)]
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

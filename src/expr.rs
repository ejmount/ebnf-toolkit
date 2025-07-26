use crate::{Rule, token_data::Span};
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

impl Expr<'_> {
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
        func: &mut impl for<'a> FnMut(&Expr<'a>) -> Option<Expr<'a>>,
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

#[cfg(test)]
mod test {
    use crate::simplification::simplify_node;
    use crate::token_data::DUMMY_SPAN;
    use proptest::prelude::*;
    use proptest::prop_oneof;

    const NAMES: [&str; 128] = const {
        const SPACING: usize = 4;
        const DIGITS: [u8; 10] = [b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9'];
        const TEMPLATE: &str = "stringXXXX";

        const BYTE_BLOCKS: [[u8; TEMPLATE.len()]; 128] = {
            let mut orig = [[0; TEMPLATE.len()]; 128];
            let mut i = 0;
            while i < orig.len() {
                let mut k = 0;
                while k < TEMPLATE.len() {
                    if k < TEMPLATE.len() - SPACING {
                        orig[i][k] = TEMPLATE.as_bytes()[k];
                    } else {
                        let diff = k - (TEMPLATE.len() - SPACING);
                        let pow = SPACING - diff - 1;
                        let shift = i / (10usize.pow(pow as u32));

                        orig[i][k] = DIGITS[shift % 10];
                    }
                    k += 1;
                }
                i += 1;
            }
            orig
        };

        let mut output = [""; 128];
        let mut a = 0;
        while a < output.len() {
            output[a] = match str::from_utf8(&BYTE_BLOCKS[a]) {
                Ok(s) => s,
                Err(_) => panic!("Whoops"),
            };
            a += 1;
        }

        output
    };

    fn node_strategy() -> impl Strategy<Value = Expr<'static>> {
        let leaf = (0..NAMES.len() * 3).prop_map(|n| {
            let typ = n % 3;
            let n = n / 3;
            match typ {
                0 => Expr::Nonterminal {
                    span: DUMMY_SPAN,
                    name: NAMES[n],
                },
                1 => Expr::Literal {
                    span: DUMMY_SPAN,
                    str: NAMES[n],
                },
                2 => Expr::Regex {
                    span: DUMMY_SPAN,
                    pattern: NAMES[n],
                },
                _ => unreachable!(),
            }
        });

        leaf.prop_recursive(2, 10, 2, |inner| {
            prop_oneof![
                prop::collection::vec(inner.clone(), 2).prop_map(|body| Expr::Choice {
                    span: DUMMY_SPAN,
                    body
                }),
                prop::collection::vec(inner.clone(), 2).prop_map(|body| Expr::Optional {
                    span: DUMMY_SPAN,
                    body
                }),
                (prop::collection::vec(inner.clone(), 2), any::<bool>()).prop_map(
                    |(body, one_needed)| Expr::Repetition {
                        span: DUMMY_SPAN,
                        body,
                        one_needed,
                    }
                ),
                prop::collection::vec(inner.clone(), 2).prop_map(|body| Expr::Group {
                    span: DUMMY_SPAN,
                    body
                }),
            ]
        })
    }

    use super::*;
    use display_tree::AsTree;

    proptest! {
        #[test]
        fn test_display(mut n in node_strategy()) {
            simplify_node(&mut n);
            let string = format!("{n}");
            let rule = format!("rule = {string};");
            let Rule { mut body, .. } = Rule::new(&rule).unwrap_or_else(|e| panic!("{e}"));
            let mut actual = body.pop().unwrap();

            simplify_node(&mut actual);

            if n != actual {
                let actual_tree = AsTree::new(&actual);
                let n_tree = AsTree::new(&n);

                eprintln!("Got: {actual}\nExpected:{n}");
                eprintln!();
                eprintln!("Trees\nGot:\n{actual_tree}\nExpected:\n{n_tree}");
                assert_eq!(actual, n);
            };
        }
    }
}

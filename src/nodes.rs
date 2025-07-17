use std::fmt::Display;

use std::sync::atomic::AtomicUsize;

use crate::{Rule, token_data::Span};
use proptest::prelude::*;
use proptest::prop_oneof;
use proptest_derive::Arbitrary;
use strum::{EnumDiscriminants, EnumProperty, IntoStaticStr, VariantNames};

#[derive(Debug, Clone, PartialEq, Eq, EnumDiscriminants)]
#[strum_discriminants(name(NodeKind), derive(VariantNames, IntoStaticStr))]
pub enum Node<'a> {
    Terminal {
        span: Span,
        str: &'a str,
    },
    Nonterminal {
        span: Span,
        name: &'a str,
    },
    Choice {
        span: Span,
        body: Vec<Node<'a>>,
    },
    Optional {
        span: Span,
        body: Vec<Node<'a>>,
    },
    Repeated {
        span: Span,
        body: Vec<Node<'a>>,
        one_needed: bool,
    },
    Regex {
        span: Span,
        pattern: &'a str,
    },
    Group {
        span: Span,
        body: Vec<Node<'a>>,
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

impl Node<'_> {
    pub fn span(&self) -> Span {
        match self {
            Node::Terminal { span, .. }
            | Node::Nonterminal { span, .. }
            | Node::Choice { span, .. }
            | Node::Optional { span, .. }
            | Node::Repeated { span, .. }
            | Node::Regex { span, .. }
            | Node::Group { span, .. }
            | Node::UnparsedOperator { span, .. }
            | Node::Rule { span, .. } => *span,
        }
    }

    pub(crate) fn node_pattern_code(&self) -> &'static str {
        if let Node::UnparsedOperator { op, .. } = self {
            op.get_str("string").unwrap()
        } else {
            let name: &str = NodeKind::from(self).into();
            &name[..1]
        }
    }

    pub(crate) fn apply_replacement(
        &mut self,
        func: &mut impl for<'a> FnMut(&Node<'a>) -> Option<Node<'a>>,
        //has_modifed: &mut bool,
    ) {
        match self {
            Node::Rule {
                span,
                rule: Rule { body, .. },
            }
            | Node::Choice { span, body }
            | Node::Optional { span, body }
            | Node::Repeated { span, body, .. }
            | Node::Group { span, body } => {
                for n in body.iter_mut() {
                    if let Some(new) = func(n) {
                        *n = new;
                    }
                    n.apply_replacement(func);
                }
                *span = body.iter().map(Node::span).reduce(Span::union).unwrap();
            }

            Node::Regex { .. }
            | Node::UnparsedOperator { .. }
            | Node::Terminal { .. }
            | Node::Nonterminal { .. } => { /* no children, do nothing */ }
        }
        let res = func(self);
        if let Some(res) = res {
            *self = res;
        }
    }
}

const NAMES: [&str; 128] = const {
    const SPACING: usize = 4;
    const DIGITS: [u8; 10] = [b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9'];
    const TEMPLATE: &str = "nonterminalXXXX";

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

static NAME_COUNTER: AtomicUsize = AtomicUsize::new(0);

// #[test]
// fn print() {
//     println!("{:?}", NAMES);
// }

fn node_strategy() -> impl Strategy<Value = Node<'static>> {
    let leaf = prop_oneof![
        any::<Span>().prop_map(|span| {
            let c = NAME_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let c = c % NAMES.len();
            Node::Nonterminal {
                span,
                name: NAMES[c],
            }
        }),
        any::<Span>().prop_map(|span| Node::Terminal {
            span,
            str: "literal"
        }),
        any::<Span>().prop_map(|span| Node::Regex {
            span,
            pattern: "regex"
        }),
        // (any::<Span>(), any::<Operator>())
        //     .prop_map(|(span, op)| Node::UnparsedOperator { span, op })
    ];

    leaf.prop_recursive(2, 10, 2, |inner| {
        prop_oneof![
            (any::<Span>(), prop::collection::vec(inner.clone(), 2))
                .prop_map(|(span, body)| Node::Choice { span, body }),
            (any::<Span>(), prop::collection::vec(inner.clone(), 2))
                .prop_map(|(span, body)| Node::Optional { span, body }),
            (
                any::<Span>(),
                prop::collection::vec(inner.clone(), 2),
                any::<bool>()
            )
                .prop_map(|(span, body, one_needed)| Node::Repeated {
                    span,
                    body,
                    one_needed,
                }),
            (any::<Span>(), prop::collection::vec(inner.clone(), 2))
                .prop_map(|(span, body)| Node::Group { span, body }),
        ]
    })
}

fn write_slice(
    f: &mut std::fmt::Formatter<'_>,
    slice: &[Node<'_>],
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

impl Display for Node<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Node::Nonterminal { name: str, .. } => write!(f, "{str}")?,
            Node::Terminal { str, .. } => write!(f, "\"{str}\"")?,

            Node::Repeated {
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
            Node::Optional { body, .. } => {
                write!(f, "[")?;
                write_slice(f, body, " ")?;
                write!(f, "]")?;
            }

            Node::Regex { pattern, .. } => write!(f, "#'{pattern}'")?,
            Node::Group { body, .. } => write_slice(f, body, " ")?,
            Node::Choice { body, .. } => {
                write_slice(f, body, "|")?;
            }
            Node::UnparsedOperator { op, .. } => write!(f, "{}", op.get_str("string").unwrap())?,
            Node::Rule {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumProperty, IntoStaticStr, Arbitrary)]
pub enum Operator {
    #[strum(props(string = "("))]
    OpenedGroup,
    #[strum(props(string = ")"))]
    ClosedGroup,
    #[strum(props(string = "["))]
    OpenedSquare,
    #[strum(props(string = "]"))]
    ClosedSquare,
    #[strum(props(string = "{"))]
    OpenedBrace,
    #[strum(props(string = "}"))]
    ClosedBrace,
    #[strum(props(string = ";"))]
    #[proptest(skip)]
    Terminator,
    #[strum(props(string = "="))]
    Equals,
    #[strum(props(string = "|"))]
    Alternation,
    #[strum(props(string = "*"))]
    Kleene,
    #[strum(props(string = "?"))]
    Optional,
    #[strum(props(string = "+"))]
    Repeat,
}

#[cfg(test)]
mod test {
    use crate::simplification::simplify_node;

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

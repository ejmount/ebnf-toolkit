use crate::{Rule, token_data::Span};

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

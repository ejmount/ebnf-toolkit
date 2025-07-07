use std::{
    ops::{ControlFlow, Range},
    slice::SliceIndex,
    str::FromStr,
    sync::LazyLock,
};

use regex::{Match, Regex};
use strum::{EnumProperty, VariantNames};

use crate::{
    error::EbnfError,
    nodes::{Node, NodeKind, NodePayload, UnparsedOperator},
    token_data::{Token, TokenPayload},
};

/// This is really just for better readibility in the raw pattern strings
fn decode_rule_regex(pat: &str) -> Regex {
    let mut s = pat.replace(' ', "");

    for name in NodeKind::VARIANTS {
        s = s.replace(name, &name[..1]);
    }
    s = s.replace("Any", NON_OPERATOR);

    s.push('$');

    Regex::new(&s).unwrap()
}
// Any node, including compound nodes, that is not an operator
const NON_OPERATOR: &str = "[A-Za-z]";

/// Regexes over the token types for each reduction rule.
/// NB: regex operators will be interpreted as usual, a grammar operator needs escaped
static REDUCTION_PATTERNS: LazyLock<[(Regex, Reducer); 7]> = LazyLock::new(|| {
    [
        (decode_rule_regex(r"Any (\| Any)+"), rules::choice),
        (decode_rule_regex(r"\[Any+\]"), rules::option),
        (decode_rule_regex(r"Any\?"), rules::option),
        (decode_rule_regex(r"Any\*"), rules::repeat),
        (decode_rule_regex(r"Any\+"), rules::repeat),
        (decode_rule_regex(r"\(Any+\)"), rules::list),
        (decode_rule_regex(r"Nonterminal = Any+;"), rules::rule),
    ]
});

type Reducer = for<'a> fn(&[Node<'a>]) -> (Node<'a>, usize);

mod rules {
    use crate::{
        nodes::{Node, NodeKind, NodePayload, UnparsedOperator},
        rule::Rule,
        token_data::Span,
    };

    fn filter_parsed<'a>(nodes: &[Node<'a>]) -> (Vec<Node<'a>>, Span, usize) {
        let size = nodes.len();
        let span = nodes.iter().map(|n| n.span).reduce(Span::union).unwrap();
        let new_nodes: Vec<Node<'_>> = nodes
            .iter()
            .filter(|n| NodeKind::UnparsedOperator != (&n.payload).into())
            .cloned()
            .collect();

        debug_assert!(!new_nodes.is_empty());
        (new_nodes, span, size)
    }

    pub(super) fn choice<'a>(nodes: &[Node<'a>]) -> (Node<'a>, usize) {
        let (new_nodes, span, size) = filter_parsed(nodes);

        let payload = NodePayload::Choice(new_nodes);
        (Node { span, payload }, size)
    }

    pub(super) fn option<'a>(nodes: &[Node<'a>]) -> (Node<'a>, usize) {
        let (new_nodes, span, size) = filter_parsed(nodes);

        let payload = NodePayload::Optional(new_nodes);
        (Node { span, payload }, size)
    }

    pub(super) fn repeat<'a>(nodes: &[Node<'a>]) -> (Node<'a>, usize) {
        let (new_nodes, span, size) = filter_parsed(nodes);

        let payload = if let NodePayload::UnparsedOperator(o) = nodes.last().unwrap().payload {
            match o {
                UnparsedOperator::Kleene | UnparsedOperator::Repeat => {
                    NodePayload::Repeated(new_nodes)
                }
                t => unreachable!(
                    "Somehow encountered {t} at the end of a repeat block - this is a bug"
                ),
            }
        } else {
            unreachable!()
        };
        (Node { span, payload }, size)
    }

    pub(super) fn list<'a>(nodes: &[Node<'a>]) -> (Node<'a>, usize) {
        let (new_nodes, span, size) = filter_parsed(nodes);

        let payload = NodePayload::List(new_nodes);
        (Node { span, payload }, size)
    }

    pub(super) fn rule<'a>(nodes: &[Node<'a>]) -> (Node<'a>, usize) {
        let size = nodes.len();
        let span = nodes.iter().map(|n| n.span).reduce(Span::union).unwrap();

        let [name, _equals, body @ .., _term] = nodes else {
            unreachable!()
        };
        let Node {
            payload: NodePayload::Nonterminal(name),
            ..
        } = name
        else {
            unreachable!(
                "Somehow picked up a Rule with a name that is not a Nonterm - this is a bug"
            )
        };

        let payload = NodePayload::Rule(Rule {
            name,
            body: body.to_vec(),
        });
        (Node { span, payload }, size)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LrStack<'a> {
    bracket_stack: Vec<(usize, Token<'a>)>,
    pub(crate) token_stack: Vec<Node<'a>>,
    pub(crate) token_pattern: String,
}

impl<'a> LrStack<'a> {
    pub(crate) fn new() -> LrStack<'a> {
        LrStack {
            bracket_stack: Vec::new(),
            token_stack: Vec::new(),
            token_pattern: String::from_str("").unwrap(),
        }
    }

    pub(crate) fn get<I: SliceIndex<[Node<'a>]>>(
        &self,
        index: I,
    ) -> Option<&<I as SliceIndex<[Node<'a>]>>::Output> {
        self.token_stack.get(index)
    }

    fn match_rule(&self, r: &Regex) -> Option<Range<usize>> {
        r.find(&self.token_pattern).as_ref().map(Match::range)
    }

    pub(crate) fn push_token(&mut self, t: Token<'a>) {
        #![allow(clippy::enum_glob_use)]
        use TokenPayload::*;
        let Token { payload, span } = t;
        let payload = match payload {
            Alternation => NodePayload::UnparsedOperator(UnparsedOperator::Alternation),
            OpeningSquare => NodePayload::UnparsedOperator(UnparsedOperator::OpenedSquare),
            ClosingSquare => NodePayload::UnparsedOperator(UnparsedOperator::ClosedSquare),
            Equals => NodePayload::UnparsedOperator(UnparsedOperator::Equals),
            Termination => NodePayload::UnparsedOperator(UnparsedOperator::Terminator),
            Kleene => NodePayload::UnparsedOperator(UnparsedOperator::Kleene),
            OpeningGroup => NodePayload::UnparsedOperator(UnparsedOperator::OpenedGroup),
            ClosingGroup => NodePayload::UnparsedOperator(UnparsedOperator::ClosedGroup),
            Optional => NodePayload::UnparsedOperator(UnparsedOperator::Optional),
            Repeat => NodePayload::UnparsedOperator(UnparsedOperator::Repeat),
            String(s) => NodePayload::Terminal(s),
            Identifier(s) => NodePayload::Nonterminal(s),
            Regex(s) => NodePayload::Regex(s),
        };
        self.push_node(Node { span, payload });
    }

    pub(crate) fn push_node(&mut self, n: Node<'a>) {
        fn node_pattern_code(n: &Node<'_>) -> &'static str {
            if let NodePayload::UnparsedOperator(o) = n.payload {
                o.get_str("string").unwrap()
            } else {
                let name: &str = NodeKind::from(&n.payload).into();
                &name[..1]
            }
        }
        //assert!(!matches!(n.payload, NodePayload::Nonterminal("tags")));
        let kind = node_pattern_code(&n);
        self.token_pattern.push_str(kind);
        self.token_stack.push(n);
    }

    pub(crate) fn peek_node(&self) -> Option<&Node<'a>> {
        self.token_stack.last()
    }

    pub(crate) fn pop_node(&mut self) -> Option<Node<'a>> {
        self.token_pattern.pop();
        self.token_stack.pop()
    }

    pub(crate) fn reduce_until_shift_needed(
        &mut self,
        lookahead: Option<&Token<'a>>,
    ) -> Result<(), Node> {
        let mut dirty = true;

        while dirty {
            dirty = false;
            for (r, f) in &*REDUCTION_PATTERNS {
                if let Some(range) = self.match_rule(r) {
                    let lookahead_pass = if let Some(t) = lookahead {
                        self.push_token(*t);
                        let r = self.match_rule(r).is_some();
                        self.pop_node();
                        r
                    } else {
                        false
                    };
                    if !lookahead_pass {
                        let nodes = self.get(range).unwrap();
                        let (replacement, consumed) = f(nodes);
                        for _ in 0..consumed {
                            self.pop_node();
                        }
                        self.push_node(replacement);
                        dirty = true;
                        #[cfg(debug_assertions)]
                        if !matches!(
                            self.token_stack.first().map(|n| &n.payload),
                            None | Some(NodePayload::Nonterminal(_) | NodePayload::Rule(_))
                        ) {
                            return Err(self.token_stack.first().unwrap().clone());
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

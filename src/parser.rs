use std::{ops::Range, slice::SliceIndex, str::FromStr, sync::LazyLock};

use regex::{Match, Regex};
use strum::VariantNames;

use crate::{
    nodes::{Node, NodeKind, Operator},
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
static REDUCTION_PATTERNS: LazyLock<[(Regex, Reducer); 8]> = LazyLock::new(|| {
    [
        (decode_rule_regex(r"Any (\| Any)+"), rules::choice),
        (decode_rule_regex(r"\[Any+\]"), rules::option),
        (decode_rule_regex(r"Any\?"), rules::option),
        (decode_rule_regex(r"Any\*"), rules::repeat),
        (decode_rule_regex(r"Any\+"), rules::repeat),
        (decode_rule_regex(r"{Any}"), rules::repeat),
        (decode_rule_regex(r"\(Any+\)"), rules::list),
        (decode_rule_regex(r"Nonterminal = Any+;"), rules::rule),
    ]
});

type Reducer = for<'a> fn(&[Node<'a>]) -> (Node<'a>, usize);

mod rules {
    use crate::{
        nodes::{Node, NodeKind, Operator},
        rule::Rule,
        token_data::Span,
    };

    fn filter_parsed<'a>(nodes: &[Node<'a>]) -> (Vec<Node<'a>>, Span, usize) {
        let size = nodes.len();
        let span = nodes.iter().map(Node::span).reduce(Span::union).unwrap();
        let new_nodes: Vec<Node<'_>> = nodes
            .iter()
            .filter(|n| NodeKind::UnparsedOperator != NodeKind::from(*n))
            .cloned()
            .collect();

        debug_assert!(!new_nodes.is_empty());
        (new_nodes, span, size)
    }

    pub(super) fn choice<'a>(nodes: &[Node<'a>]) -> (Node<'a>, usize) {
        let (body, span, size) = filter_parsed(nodes);

        (Node::Choice { span, body }, size)
    }

    pub(super) fn option<'a>(nodes: &[Node<'a>]) -> (Node<'a>, usize) {
        let (body, span, size) = filter_parsed(nodes);

        (Node::Optional { span, body }, size)
    }

    pub(super) fn repeat<'a>(nodes: &[Node<'a>]) -> (Node<'a>, usize) {
        let (body, span, size) = filter_parsed(nodes);

        let node = if let Node::UnparsedOperator { op, .. } = nodes.last().unwrap() {
            match op {
                Operator::Kleene => Node::Repeated {
                    span,
                    body,
                    one_needed: false,
                },
                Operator::ClosedBrace | Operator::Repeat => Node::Repeated {
                    span,
                    body,
                    one_needed: true,
                },
                t => unreachable!("Encountered {t:?} at the end of a repeat block - this is a bug"),
            }
        } else {
            unreachable!()
        };
        (node, size)
    }

    pub(super) fn list<'a>(nodes: &[Node<'a>]) -> (Node<'a>, usize) {
        let (body, span, size) = filter_parsed(nodes);

        (Node::Group { span, body }, size)
    }

    pub(super) fn rule<'a>(nodes: &[Node<'a>]) -> (Node<'a>, usize) {
        let size = nodes.len();
        let span = nodes.iter().map(Node::span).reduce(Span::union).unwrap();

        let [Node::Nonterminal { name, .. }, _equals, body @ .., _term] = nodes else {
            unreachable!("Bug: Rule with a name of {:?}", nodes.first())
        };

        let rule_node = Node::Rule {
            span,
            rule: Rule {
                name,
                body: body.to_vec(),
            },
        };
        (rule_node, size)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LrStack<'a> {
    bracket_stack: Vec<(usize, Token<'a>)>,
    token_stack: Vec<Node<'a>>,
    token_pattern: String,
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
        use Operator as Op;
        #[allow(clippy::enum_glob_use)]
        use TokenPayload::*;
        let Token { payload, span } = t;
        let op_node = |op| Node::UnparsedOperator { op, span };
        let node = match payload {
            Alternation => op_node(Op::Alternation),
            OpeningBrace => op_node(Op::OpenedBrace),
            ClosingBrace => op_node(Op::ClosedBrace),
            OpeningSquare => op_node(Op::OpenedSquare),
            ClosingSquare => op_node(Op::ClosedSquare),
            Equals => op_node(Op::Equals),
            Termination => op_node(Op::Terminator),
            Kleene => op_node(Op::Kleene),
            OpeningGroup => op_node(Op::OpenedGroup),
            ClosingGroup => op_node(Op::ClosedGroup),
            Optional => op_node(Op::Optional),
            Repeat => op_node(Op::Repeat),
            String(s) => Node::Terminal { span, str: s },
            Identifier(s) => Node::Nonterminal { span, name: s },
            Regex(s) => Node::Regex { span, pattern: s },
            Newline => unreachable!(),
        };
        self.push_node(node);
    }

    pub(crate) fn push_node(&mut self, n: Node<'a>) {
        self.token_pattern.push_str(n.node_pattern_code());
        self.token_stack.push(n);
    }

    pub(crate) fn peek_node(&self) -> Option<&Node<'a>> {
        self.token_stack.last()
    }

    pub(crate) fn pop_node(&mut self) -> Option<Node<'a>> {
        self.token_pattern.pop();
        self.token_stack.pop()
    }

    /// Repeatedly reduce using the defined patterns until no more reductions can be made
    ///
    /// **NB** Assumes that a found match cannot be potentially extended with more input and still match.
    ///
    /// This does not apply to (A|B|C...) because "A|B|" is invalid without looking even further ahead to the C
    /// For now its fine to have a stack of binary operators and simplify later.
    pub(crate) fn reduce_until_shift_needed(&mut self) -> Result<(), (&Self, Node<'a>)> {
        let mut dirty = true;

        #[cfg(any(debug_assertions, test))]
        if !matches!(
            self.token_stack.first().map(NodeKind::from),
            None | Some(NodeKind::Nonterminal | NodeKind::Rule)
        ) {
            return Err((self, self.token_stack.first().unwrap().clone()));
        }
        while dirty {
            dirty = false;
            for (r, f) in &*REDUCTION_PATTERNS {
                if let Some(range) = self.match_rule(r) {
                    let nodes = self.get(range).unwrap();
                    let (replacement, consumed) = f(nodes);
                    for _ in 0..consumed {
                        self.pop_node();
                    }
                    self.push_node(replacement);
                    dirty = true;
                }
            }
        }
        Ok(())
    }
}

use std::{ops::Range, slice::SliceIndex, str::FromStr};

use regex::{Match, Regex};
use strum::{EnumProperty, VariantNames};
use winnow::{
    LocatingSlice,
    stream::{Stream, TokenSlice},
};

use crate::{
    container::MyVec as Vec,
    lexing::tokenize,
    nodes::{Node, NodeKind, NodePayload, UnparsedOperator},
    token_data::{LexedInput, Token, TokenPayload, TokenStore},
};

const ANY_PATTERN: &str = "[^U]";

const PATTERNS: [(&str, Reducer); 7] = const {
    [
        (r"Any (\| Any)+", rules::choice),
        (r"\[Any+\]", rules::option),
        (r"Any\?", rules::option),
        (r"Any\*", rules::repeat),
        (r"Any\+", rules::repeat),
        (r"\(Any+\)", rules::list),
        (r"Nonterminal = Any+;", rules::rule),
    ]
};

type Reducer = for<'a> fn(&[Node<'a>]) -> (Node<'a>, usize);

fn decode_rule_regex(pat: &str) -> Regex {
    let mut s = pat.replace(' ', "");

    for name in NodeKind::VARIANTS {
        s = s.replace(name, &name[..1]);
    }
    s = s.replace("Any", ANY_PATTERN);

    s.push('$');

    Regex::new(&s).unwrap()
}

mod rules {
    use crate::{
        container::MyVec as Vec,
        nodes::{Node, NodeKind, NodePayload, Rule, UnparsedOperator},
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
        //let

        let [name, _equals, body @ .., _term] = nodes else {
            unreachable!()
        };

        let payload = NodePayload::Rule(Rule {
            name: Box::new(name.clone()),
            body: body.iter().cloned().collect(),
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

    pub(crate) fn match_rule(&self, r: &Regex) -> Option<Range<usize>> {
        r.find(&self.token_pattern).as_ref().map(Match::range)
    }

    pub(crate) fn push_token(&mut self, t: Token<'a>) {
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
            Whitespace(_) | Newline => unreachable!(),
        };
        self.push_node(Node { span, payload });
    }

    pub(crate) fn push_node(&mut self, n: Node<'a>) {
        let kind = if let NodePayload::UnparsedOperator(o) = n.payload {
            o.get_str("string").unwrap()
        } else {
            let nk = NodeKind::from(&n.payload);
            let name: &str = nk.into();
            &name[..1]
        };
        self.token_pattern.push_str(kind);
        self.token_stack.push(n);
    }

    fn pop_node(&mut self) -> Option<Node<'a>> {
        self.token_pattern.pop();
        self.token_stack.pop()
    }

    pub(crate) fn drop_many(&mut self, n: usize) {
        for _ in 0..n {
            self.pop_node();
        }
    }

    fn shiftreduce(&mut self, input: &mut LexedInput<'a, '_>) {
        let regex_pattern = PATTERNS.map(|(p, f)| (decode_rule_regex(p), f));

        loop {
            let mut shift = true;
            for (r, f) in &regex_pattern {
                if let Some(range) = self.match_rule(r) {
                    let lookahead_pass = if let Some(t) = input.first() {
                        self.push_token(*t);
                        let r = self.match_rule(r).is_some();
                        self.drop_many(1);
                        r
                    } else {
                        false
                    };
                    if !lookahead_pass {
                        let nodes = self.get(range).unwrap();
                        let (replacement, consumed) = f(nodes);
                        self.drop_many(consumed);
                        self.push_node(replacement);
                        shift = false;
                        break;
                    }
                }
            }
            if shift {
                if let Some(t) = input.next_token() {
                    self.push_token(*t);
                } else {
                    break;
                }
            }
        }
    }
}

pub(crate) fn file_reduce() {
    let file = include_str!(r"..\tests\irc.ebnf");

    for l in file.lines() {
        dbg!(l);
        let locating = LocatingSlice::new(l);
        let TokenStore(tokens) = tokenize(locating).unwrap();
        let mut input = TokenSlice::new(tokens.get(..).unwrap());
        let mut stack = LrStack::new();
        stack.shiftreduce(&mut input);
        for n in stack.get(..).unwrap() {
            display_tree::println_tree!(*n);
        }
    }
}

#[cfg(test)]
mod tests {
    use winnow::{LocatingSlice, stream::TokenSlice};

    use crate::{
        lexing::tokenize,
        parser::{PATTERNS, decode_rule_regex, shiftreduce},
        token_data::TokenStore,
    };

    #[test]
    fn decode_test() {
        for (k, p) in PATTERNS {
            println!("{k} => {}", decode_rule_regex(k).as_str());
        }
    }

    // #[test]
    // fn reduce_test() {
    //     //let src = "  'A' | 'B' | 'C'  ";
    //     let src = "A = ['A' 'B' 'C'];";
    //     let locating = LocatingSlice::new(src);
    //     let TokenStore(tokens) = tokenize(locating).unwrap();

    //     let mut input = TokenSlice::new(tokens.get(..).unwrap());

    //     let mut stack = LrStack::new();

    //     shiftreduce(&mut stack, &mut input);

    //     for n in stack.get(..).unwrap() {
    //         display_tree::println_tree!(*n);
    //     }
    // }
}

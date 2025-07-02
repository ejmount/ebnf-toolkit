use regex::Regex;
use strum::VariantNames;
use winnow::{
    LocatingSlice,
    stream::{Stream, TokenSlice},
};

use crate::{
    container::MyVec as Vec,
    lexing::tokenize,
    nodes::{LrStack, Node, NodeKind, NodePayload, Rule},
    token_data::{LexedInput, Span, TokenKind, TokenStore},
};

const ANY_PATTERN: &str = "[^U]";

type Reducer = for<'a> fn(&[Node<'a>]) -> (Node<'a>, usize);

fn filter_parsed<'a>(nodes: &[Node<'a>]) -> (Vec<Node<'a>>, Span, usize) {
    let size = nodes.len();
    let span = nodes.iter().map(|n| n.span).reduce(Span::union).unwrap();
    let new_nodes: Vec<Node<'_>> = nodes
        .iter()
        .filter(|n| NodeKind::UnparsedToken != (&n.payload).into())
        .cloned()
        .collect();

    debug_assert!(!new_nodes.is_empty());
    (new_nodes, span, size)
}

fn choice<'a>(nodes: &[Node<'a>]) -> (Node<'a>, usize) {
    let (new_nodes, span, size) = filter_parsed(nodes);

    let payload = NodePayload::Choice(new_nodes);
    (Node { span, payload }, size)
}

fn option<'a>(nodes: &[Node<'a>]) -> (Node<'a>, usize) {
    let (new_nodes, span, size) = filter_parsed(nodes);

    let payload = NodePayload::Optional(new_nodes);
    (Node { span, payload }, size)
}

fn repeat<'a>(nodes: &[Node<'a>]) -> (Node<'a>, usize) {
    let (new_nodes, span, size) = filter_parsed(nodes);

    let payload = if let NodePayload::UnparsedToken(t) = nodes.last().unwrap().payload {
        match TokenKind::from(t) {
            TokenKind::Kleene | TokenKind::Repeat => NodePayload::Repeated(new_nodes),
            t => unreachable!("{t}"),
        }
    } else {
        unreachable!()
    };
    (Node { span, payload }, size)
}

fn list<'a>(nodes: &[Node<'a>]) -> (Node<'a>, usize) {
    let (new_nodes, span, size) = filter_parsed(nodes);

    let payload = NodePayload::List(new_nodes);
    (Node { span, payload }, size)
}

fn decode_rule_regex(pat: &str) -> Regex {
    let mut s = pat.replace(' ', "");

    for name in NodeKind::VARIANTS {
        s = s.replace(name, &name[..1]);
    }
    s = s.replace("Any", ANY_PATTERN);

    s.push('$');

    Regex::new(&s).unwrap()
}

fn rule<'a>(nodes: &[Node<'a>]) -> (Node<'a>, usize) {
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

const PATTERNS: [(&str, Reducer); 7] = const {
    [
        (r"Any (\| Any)+", choice),
        (r"\[Any+\]", option),
        (r"Any\?", option),
        (r"Any\*", repeat),
        (r"Any\+", repeat),
        (r"\(Any+\)", list),
        (r"Nonterminal = Any+;", rule),
    ]
};

fn shiftreduce<'a>(stack: &mut LrStack<'a>, input: &mut LexedInput<'a, '_>) {
    let regex_pattern = PATTERNS.map(|(p, f)| (decode_rule_regex(p), f));

    loop {
        let mut shift = true;
        for (r, f) in &regex_pattern {
            //dbg!(&r.as_str());
            if let Some(range) = stack.match_rule(r) {
                //dbg!(r.as_str(), &range);
                let lookahead_pass = if let Some(t) = input.first() {
                    stack.push_token(*t);
                    let r = stack.match_rule(r).is_some();
                    stack.drop_many(1);
                    r
                } else {
                    false
                };
                if !lookahead_pass {
                    // dbg!(&stack.token_stack.len());
                    // dbg!(&stack.token_pattern);
                    let nodes = stack.get(range).unwrap();
                    //dbg!(&nodes);
                    let (replacement, consumed) = f(nodes);
                    stack.drop_many(consumed);
                    stack.push_node(replacement);
                    shift = false;
                    break;
                }
            }
        }
        if shift {
            if let Some(t) = input.next_token() {
                stack.push_token(*t);
            } else {
                break;
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
        shiftreduce(&mut stack, &mut input);
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
        nodes::LrStack,
        parser::{PATTERNS, decode_rule_regex, shiftreduce},
        token_data::TokenStore,
    };

    #[test]
    fn decode_test() {
        for (k, p) in PATTERNS {
            println!("{k} => {}", decode_rule_regex(k).as_str())
        }
    }

    #[test]
    fn reduce_test() {
        //let src = "  'A' | 'B' | 'C'  ";
        let src = "A = ['A' 'B' 'C'];";
        let locating = LocatingSlice::new(src);
        let TokenStore(tokens) = tokenize(locating).unwrap();

        let mut input = TokenSlice::new(tokens.get(..).unwrap());

        let mut stack = LrStack::new();

        shiftreduce(&mut stack, &mut input);

        for n in stack.get(..).unwrap() {
            display_tree::println_tree!(*n);
        }
    }
}

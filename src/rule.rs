use std::collections::{HashMap, VecDeque};

use crate::{
    Node, error::EbnfError, nodes::NodePayload, parse_rule_from_tokens, token_data::tokenize,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule<'a> {
    pub name: &'a str,
    pub body: Vec<Node<'a>>,
}

impl<'a> Rule<'a> {
    pub fn nonterminals(&self) -> Vec<&'a str> {
        #[allow(clippy::enum_glob_use)]
        use NodePayload::*;
        let mut stack: VecDeque<_> = self.body.iter().collect();
        let mut nonterm_names = vec![];

        while let Some(node) = stack.pop_front() {
            match &node.payload {
                Regex(_) | Terminal(_) => {}
                UnparsedOperator(_) | Rule(_) => unreachable!(),
                Choice(nodes) | Optional(nodes) | Repeated(nodes) | List(nodes) => {
                    stack.extend(nodes);
                }
                Nonterminal(s) => nonterm_names.push(*s),
            }
        }
        nonterm_names
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Grammar<'a> {
    pub rules: HashMap<&'a str, Rule<'a>>,
}

impl Grammar<'_> {
    pub fn new(input: &str) -> Result<Grammar<'_>, EbnfError<'_>> {
        let tokens = tokenize(input)?;
        eprintln!("{:?}", &tokens[14..20]);

        let mut tokens_buffer = &tokens[..];
        let rules = parse_rule_from_tokens(input, &mut tokens_buffer)?;
        Ok(rules.into_iter().collect())
    }

    pub fn get_dangling_reference(&self) -> Option<(&str, &str)> {
        for rule in self.rules.values() {
            let refers = rule.nonterminals();
            for r in refers {
                if !self.rules.contains_key(r) {
                    return Some((rule.name, r));
                }
            }
        }
        None
    }
}

impl<'a> FromIterator<Rule<'a>> for Grammar<'a> {
    fn from_iter<T: IntoIterator<Item = Rule<'a>>>(iter: T) -> Self {
        let mut g = Self::default();
        for r in iter {
            g.rules.insert(r.name, r);
        }
        g
    }
}

#[cfg(test)]
mod test {
    use crate::{Node, Span, nodes::NodePayload};

    #[test]
    fn nonterminals_order() {
        #[allow(clippy::enum_glob_use)]
        use crate::nodes::NodePayload::*;
        let span = Span { start: 0, end: 0 };
        let body = vec![
            Node {
                span,
                payload: List(vec![
                    Node {
                        span,
                        payload: Nonterminal("A"),
                    },
                    Node {
                        span,
                        payload: Nonterminal("B"),
                    },
                ]),
            },
            Node {
                span,
                payload: Nonterminal("C"),
            },
        ];

        let nonterms = super::Rule { body, name: "" }.nonterminals();
        insta::assert_compact_debug_snapshot!(nonterms, @r#"["C", "A", "B"]"#);
    }
}

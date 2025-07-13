use std::collections::{HashMap, VecDeque};

use crate::{Node, error::EbnfError, parse_rule_from_tokens, token_data::tokenize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule<'a> {
    pub name: &'a str,
    pub body: Vec<Node<'a>>,
}

impl<'a> Rule<'a> {
    pub fn new(input: &str) -> Result<Rule<'_>, EbnfError> {
        let tokens = tokenize(input)?;

        let mut tokens_buffer = &tokens[..];
        parse_rule_from_tokens(input, &mut tokens_buffer)?
            .into_iter()
            .next()
            .ok_or(EbnfError::EmptyInput)
    }

    pub fn nonterminals(&self) -> Vec<&'a str> {
        #[allow(clippy::enum_glob_use)]
        use Node::*;
        let mut stack: VecDeque<_> = self.body.iter().collect();
        let mut nonterm_names = vec![];

        while let Some(node) = stack.pop_front() {
            match node {
                Regex { .. } | Terminal { .. } | UnparsedOperator { .. } => {}
                Nonterminal { name, .. } => nonterm_names.push(*name),
                Choice { body, .. }
                | Optional { body, .. }
                | Repeated { body, .. }
                | List { body, .. } => stack.extend(body),

                Rule { .. } => unreachable!(),
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
        //eprintln!("{:?}", &tokens[14..20]);

        let mut tokens_buffer = &tokens[..];
        let rules = parse_rule_from_tokens(input, &mut tokens_buffer)?;
        Ok(rules.into_iter().collect())
    }

    pub fn first_dangling_reference(&self) -> Option<(&str, &str)> {
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
    use crate::{Grammar, Node, Rule, token_data::DUMMY_SPAN};

    #[test]
    fn nonterminals() {
        let span = DUMMY_SPAN;
        let body = vec![
            Node::List {
                span,
                body: vec![
                    Node::Nonterminal { span, name: "A" },
                    Node::Nonterminal { span, name: "B" },
                ],
            },
            Node::Nonterminal { span, name: "C" },
        ];

        let nonterms = Rule { body, name: "" }.nonterminals();
        insta::assert_compact_debug_snapshot!(nonterms, @r#"["C", "A", "B"]"#);
    }

    #[test]
    fn nonterminals_nested() {
        let src = "Foo = (A|#'Hello'|'Goodbye'|B?)*;";
        let nonterms = Rule::new(src).unwrap().nonterminals();
        insta::assert_compact_debug_snapshot!(nonterms, @r#"["B", "A"]"#);
    }

    #[test]
    fn dangling_refs() {
        let src = "A = B;";
        let g = Grammar::new(src).unwrap();
        let first_dangling = g.first_dangling_reference();
        insta::assert_compact_debug_snapshot!(first_dangling, @r#"Some(("A", "B"))"#);
    }
}

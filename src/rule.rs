use std::collections::{HashMap, VecDeque};

use crate::{Expr, error::EbnfError, parse_rules_from_tokens, token_data::tokenize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule<'a> {
    pub name: &'a str,
    pub body: Vec<Expr<'a>>,
}

impl<'a> Rule<'a> {
    pub fn new(input: &str) -> Result<Rule<'_>, EbnfError> {
        let tokens = tokenize(input)?;

        let mut tokens_buffer = &tokens[..];
        parse_rules_from_tokens(input, &mut tokens_buffer)?
            .into_iter()
            .next()
            .ok_or(EbnfError::EmptyInput)
    }

    pub fn nonterminals(&self) -> Vec<&'a str> {
        #[allow(clippy::enum_glob_use)]
        use Expr::*;
        let mut stack: VecDeque<_> = self.body.iter().collect();
        let mut nonterm_names = vec![];

        while let Some(node) = stack.pop_front() {
            match node {
                Regex { .. } | Literal { .. } | UnparsedOperator { .. } => {}
                Nonterminal { name, .. } => nonterm_names.push(*name),
                Choice { body, .. }
                | Optional { body, .. }
                | Repetition { body, .. }
                | Rule {
                    rule: crate::Rule { body, .. }, // Shouldn't be possible in practice but might as well cover it
                    ..
                }
                | Group { body, .. } => stack.extend(body),
            }
        }
        nonterm_names
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Grammar<'a> {
    rules: HashMap<&'a str, Rule<'a>>,
}

impl Grammar<'_> {
    pub fn new(input: &str) -> Result<Grammar<'_>, EbnfError<'_>> {
        let tokens = tokenize(input)?;
        let rules = parse_rules_from_tokens(input, &mut &tokens[..])?;
        Ok(rules.into_iter().collect())
    }

    pub fn get<Q>(&self, i: Q) -> Option<&Rule>
    where
        &'a str: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.rules.get(&i)
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

impl<'a, B> Index<B> for Grammar<'a>
where
    B: Borrow<str>,
{
    type Output = Rule<'a>;
    fn index(&self, index: B) -> &Self::Output {
        let s = index.borrow();
        &self.rules[s]
    }
}

impl<'a> FromIterator<Rule<'a>> for Grammar<'a> {
    fn from_iter<T: IntoIterator<Item = Rule<'a>>>(iter: T) -> Self {
        let mut rules: HashMap<&'a str, Rule<'a>> = HashMap::new();
        for (name, new_rule) in iter.into_iter().map(|r| (r.name, r)) {
            if let hash_map::Entry::Occupied(o) = rules.entry(name) {
                let old_rule = o.remove();
                rules.insert(name, merge_duplicate_rule(old_rule, new_rule));
            } else {
                rules.insert(name, new_rule);
            }
        }

        Grammar { rules }
    }
}

fn merge_duplicate_rule<'a>(old_rule: Rule<'a>, new_rule: Rule<'a>) -> Rule<'a> {
    fn unwrap_choice_items(mut e: Vec<Expr<'_>>) -> Vec<Expr<'_>> {
        if e.len() == 1
            && let Some(Expr::Choice { .. }) = e.first()
        {
            let Some(Expr::Choice { body, .. }) = e.pop() else {
                unreachable!()
            };
            body
        } else {
            e
        }
    }

    let old_body = unwrap_choice_items(old_rule.body);
    let new_body = unwrap_choice_items(new_rule.body);
    let body: Vec<_> = old_body.into_iter().chain(new_body).collect();
    let span = Span::union(body.iter());

    Rule {
        name: old_rule.name,
        body: vec![Expr::Choice { span, body }],
    }
}

#[cfg(test)]
mod test {
    use crate::{Expr, Grammar, Rule, token_data::DUMMY_SPAN};
    use display_tree::AsTree;

    #[test]
    fn nonterminals() {
        let span = DUMMY_SPAN;
        let body = vec![
            Expr::Group {
                span,
                body: vec![
                    Expr::Nonterminal { span, name: "A" },
                    Expr::Nonterminal { span, name: "B" },
                ],
            },
            Expr::Nonterminal { span, name: "C" },
        ];

        let nonterms = Rule { body, name: "" }.nonterminals();
        insta::assert_compact_debug_snapshot!(nonterms, @r#"["C", "A", "B"]"#);
    }

    #[test]
    fn nonterminals_nested() {
        let src = "Foo = (A|#'Hello'|'Goodbye'|B?)*;";
        let nonterms = Rule::new(src).unwrap().nonterminals();
        insta::assert_compact_debug_snapshot!(nonterms, @r#"["A", "B"]"#);
    }

    #[test]
    fn duplicate_names() {
        let src = "A = B; A = C; B = A|B; B = C; C = A; C = B|C;  D = C|D; D = A|B;";
        let g = Grammar::new(src).unwrap();
        insta::assert_snapshot!(AsTree::new(&g.rules["A"]), @r"
        Rule
        ├─name: A
        └─0: Choice [1:4..1:12]
             └─0: Nonterminal [1:4..1:5]
               │  └─ B
               1: Nonterminal [1:11..1:12]
                  └─ C
        ");
        insta::assert_snapshot!(AsTree::new(&g.rules["B"]), @r"
        Rule
        ├─name: B
        └─0: Choice [1:18..1:28]
             └─0: Nonterminal [1:18..1:19]
               │  └─ A
               1: Nonterminal [1:20..1:21]
               │  └─ B
               2: Nonterminal [1:27..1:28]
                  └─ C
        ");
        insta::assert_snapshot!(AsTree::new(&g.rules["C"]), @r"
        Rule
        ├─name: C
        └─0: Choice [1:34..1:44]
             └─0: Nonterminal [1:34..1:35]
               │  └─ A
               1: Nonterminal [1:41..1:42]
               │  └─ B
               2: Nonterminal [1:43..1:44]
                  └─ C
        ");
        insta::assert_snapshot!(AsTree::new(&g.rules["D"]), @r"
        Rule
        ├─name: D
        └─0: Choice [1:51..1:63]
             └─0: Nonterminal [1:51..1:52]
               │  └─ C
               1: Nonterminal [1:53..1:54]
               │  └─ D
               2: Nonterminal [1:60..1:61]
               │  └─ A
               3: Nonterminal [1:62..1:63]
                  └─ B
        ");
    }
}

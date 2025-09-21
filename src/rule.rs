use std::{
    borrow::Cow,
    collections::{HashMap, VecDeque},
    hash::Hash,
    ops::Index,
};

use crate::{Expr, Span, error::EbnfError, parse_rules_from_tokens, token_data::tokenize};

/// A single production rule of a grammar. Will generally be an intermediate step on the way to either creating a [`Grammar`] or analysing the rule's `body`, which represents an ordered sequence of [`Expr`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Rule<'a> {
    /// The name of this rule - using [`Rule::new`] or [`Grammar::new`] will borrow this from the input data, but an owned String can also be used
    pub name: Cow<'a, str>,
    /// The sequence of nodes that the name refers to. Semantically equivalent to a [`Expr::Group`]
    pub body: Vec<Expr<'a>>,
}

impl<'a> Rule<'a> {
    /// Parses a rule from an input string
    ///
    /// # Errors
    /// If the input string is ill-formed, an [`EbnfError`] is returned. See that type for possible reasons why.
    pub fn new(input: &str) -> Result<Rule<'_>, EbnfError<'_>> {
        let tokens = tokenize(input)?;

        let mut tokens_buffer = &tokens[..];
        parse_rules_from_tokens(input, &mut tokens_buffer)?
            .into_iter()
            .next()
            .ok_or(EbnfError::EmptyInput)
    }

    /// Returns a list of all the nonterminal names that appear anywhere within this rule
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

    /// Does this rule contain any reference to itself?
    pub fn is_recursive(&self) -> bool {
        self.nonterminals().contains(&&*self.name)
    }

    /// Whether this rule refers to any other rules or is entirely self-contained
    pub fn contains_any_nonterminal(&self) -> bool {
        self.body.iter().any(Expr::contains_nonterminal)
    }
}

/// A set of EBNF rules
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Grammar<'a> {
    rules: HashMap<Cow<'a, str>, Rule<'a>>,
}

impl Grammar<'_> {
    /// Parses a grammar - a sequence of [`Rule`]s from an input string.
    ///
    /// # Errors
    /// If the input string is ill-formed, an [`EbnfError`] is returned. See that type for possible reasons.
    pub fn new(input: &str) -> Result<Grammar<'_>, EbnfError<'_>> {
        let tokens = tokenize(input)?;
        let rules = parse_rules_from_tokens(input, &mut &tokens[..])?;
        Ok(rules.into_iter().collect())
    }

    /// Gets the rule by a given name. The [`Index`] trait is also available to instead panic if the name is not found
    pub fn get(&self, name: &str) -> Option<&Rule<'_>> {
        self.rules.get(name)
    }

    /// Tests if any of the rules contain a nonterminal name that does not have a corresponding entry in this `Grammar`.
    /// If one exists, returns the name of the rule containing the nonterminal, and the name of the missing rule itself. Else returns `None`.
    /// ```rust
    /// # use ebnf_toolkit::Grammar;
    /// let src = "A = B;";
    /// let g = Grammar::new(src).unwrap();
    /// assert_eq!(g.first_dangling_reference(), Some(("A", "B")));
    ///
    /// let recurse = "A = A;";
    /// let g2 = Grammar::new(recurse).unwrap();
    /// assert_eq!(g2.first_dangling_reference(), None);
    /// ```
    pub fn first_dangling_reference(&self) -> Option<(&str, &str)> {
        for rule in self.rules.values() {
            let refers = rule.nonterminals();
            for r in refers {
                if !self.rules.contains_key(r) {
                    return Some((&*rule.name, r));
                }
            }
        }
        None
    }
}

impl<'a> Index<&str> for Grammar<'a> {
    type Output = Rule<'a>;
    fn index(&self, index: &str) -> &Self::Output {
        &self.rules[index]
    }
}

impl<'a> FromIterator<Rule<'a>> for Grammar<'a> {
    fn from_iter<T: IntoIterator<Item = Rule<'a>>>(iter: T) -> Self {
        let mut rules: HashMap<Cow<'a, str>, Rule<'a>> = HashMap::new();
        for new_rule in iter {
            if let Some(old_rule) = rules.remove(&new_rule.name) {
                let new_body = merge_duplicate_rule(old_rule.body, new_rule.body);
                let combined_rule = Rule {
                    name: old_rule.name,
                    body: new_body,
                };
                rules.insert(combined_rule.name.clone(), combined_rule);
            } else {
                rules.insert(new_rule.name.clone(), new_rule);
            }
        }

        Grammar { rules }
    }
}

fn merge_duplicate_rule<'a>(old_rule: Vec<Expr<'a>>, new_rule: Vec<Expr<'a>>) -> Vec<Expr<'a>> {
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

    let old_body = unwrap_choice_items(old_rule);
    let new_body = unwrap_choice_items(new_rule);
    let body: Vec<_> = old_body.into_iter().chain(new_body).collect();
    let span = Span::union(body.iter());

    vec![Expr::Choice { span, body }]
}

#[cfg(test)]
mod test {
    use std::borrow::Cow;

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

        let nonterms = Rule {
            body,
            name: Cow::Borrowed(""),
        }
        .nonterminals();
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

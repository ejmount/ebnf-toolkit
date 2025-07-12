#![warn(warnings)]
use std::{
    fmt::{Formatter, Write},
    iter::once,
};

use display_tree::{AsTree, DisplayTree, Style};

use crate::{Node, Rule, nodes::NodeKind};

const EMPTY_STRING: &str = "";

impl DisplayTree for Node<'_> {
    fn fmt(&self, f: &mut Formatter, style: Style) -> std::fmt::Result {
        let indentation = style.indentation as usize - 1;
        let horizontal_bar = format!("{:indentation$}", style.char_set.horizontal);

        if NodeKind::from(self) != NodeKind::Rule {
            let name: &str = NodeKind::from(self).into();
            writeln!(f, "{} {}", style.leaf_style.apply(name), self.span())?;
        }

        match self {
            Node::Terminal { str, .. } => write!(
                f,
                "{}",
                style.branch_style.apply(&format!(
                    "{}{horizontal_bar} '{}'",
                    style.char_set.end_connector,
                    str.escape_debug()
                ))
            )?,
            Node::Regex { pattern: s, .. } | Node::Nonterminal { name: s, .. } => write!(
                f,
                "{}",
                style.branch_style.apply(&format!(
                    "{}{horizontal_bar} {}",
                    style.char_set.end_connector, s
                ))
            )?,
            Node::UnparsedOperator { op, .. } => {
                let op: &str = op.into();
                write!(
                    f,
                    "{}",
                    style.branch_style.apply(&format!(
                        "{}{horizontal_bar} {}",
                        style.char_set.end_connector, op
                    ))
                )?;
            }

            Node::Choice { body, .. }
            | Node::Optional { body, .. }
            | Node::Repeated { body, .. }
            | Node::List { body, .. } => {
                print_vec_tree(f, style, body)?;
            }
            Node::Rule { rule, .. } => write!(f, "{}", AsTree::new(rule))?,
        }
        Ok(())
    }
}

pub(crate) fn print_vec_tree(
    f: &mut impl Write,
    style: Style,
    body: &[Node<'_>],
) -> Result<(), std::fmt::Error> {
    let indentation = style.indentation as usize - 1;
    let spacer = format!(" {EMPTY_STRING:indentation$}");
    let horizontal_bar = format!("{:indentation$}", style.char_set.horizontal);
    let vec_output = fmt_vec(body, style);

    for (block_no, block) in vec_output.into_iter().enumerate() {
        for (n, line) in block.lines().enumerate() {
            if n == 0 && block_no == 0 {
                write!(f, "{}{horizontal_bar}", style.char_set.end_connector,)?;
            } else {
                write!(f, "{spacer}")?;
            }
            writeln!(f, "{line}")?;
        }
    }
    Ok(())
}

impl DisplayTree for Rule<'_> {
    fn fmt(&self, f: &mut Formatter, style: Style) -> std::fmt::Result {
        let indentation = style.indentation as usize - 1;
        let horizontal_bar = format!("{:indentation$}", style.char_set.horizontal);
        writeln!(f, "{}", style.leaf_style.apply("Rule"))?;
        writeln!(
            f,
            "{1}{horizontal_bar}name: {0}",
            &self.name, style.char_set.connector
        )?;
        //let vec_output = fmt_vec(&self.body, style);

        print_vec_tree(f, style, &self.body)?;
        Ok(())
    }
}

pub(crate) fn fmt_vec<T: DisplayTree>(v: &[T], style: Style) -> impl Iterator<Item = String> + '_ {
    let max_index = v.len() - 1;
    let num_width = format!("{max_index}",).len();

    let vertical = style
        .branch_style
        .apply(&style.char_set.vertical.to_string());

    v.iter().enumerate().map(move |(n, item)| {
        let vertical = vertical.clone();
        let continued_vertical = if n < max_index { &vertical } else { " " };

        let indent = format!(" {:num_width$}", "");
        let tree = AsTree::with_style(item, style).to_string();
        let mut tree_lines = tree.lines().enumerate().map(move |(line_num, line)| {
            let line = style.leaf_style.apply(line);
            if line_num > 0 {
                format!("{continued_vertical}{indent}{line}\n")
            } else {
                format!("{line}\n")
            }
        });
        let lead_line = tree_lines.next().unwrap_or(String::new());

        let lead = format!("{n:<0num_width$}: {lead_line}");

        once(lead).chain(tree_lines).collect()
    })
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::{nodes::Operator, token_data::DUMMY_SPAN};
    #[test]
    fn one_level_test() {
        let span = DUMMY_SPAN;

        let body = vec![
            Node::Nonterminal {
                span,
                name: "Nonterm",
            },
            Node::Terminal { span, str: "Term" },
            Node::UnparsedOperator {
                span,
                op: Operator::Equals,
            },
            Node::Choice {
                span,
                body: vec![
                    Node::Optional {
                        span,
                        body: vec![Node::Regex { span, pattern: "." }],
                    },
                    Node::Repeated {
                        span,
                        body: vec![Node::Regex { span, pattern: "a" }],
                        one_needed: true,
                    },
                ],
            },
        ];
        let n = Node::Rule {
            span,
            rule: Rule { name: "name", body },
        };
        let tree = AsTree::new(&n);

        insta::assert_snapshot!(tree, @r"
        Rule
        ├─name: name
        └─0: Nonterminal [4294967294:0..4294967294:2]
          │  └─ Nonterm
          1: Terminal [4294967294:0..4294967294:2]
          │  └─ 'Term'
          2: UnparsedOperator [4294967294:0..4294967294:2]
          │  └─ Equals
          3: Choice [4294967294:0..4294967294:2]
             └─0: Optional [4294967294:0..4294967294:2]
               │  └─0: Regex [4294967294:0..4294967294:2]
               │       └─ .
               1: Repeated [4294967294:0..4294967294:2]
                  └─0: Regex [4294967294:0..4294967294:2]
                       └─ a
        ");
    }

    #[test]
    fn long_list_test() {
        let span = DUMMY_SPAN;
        let strings: Vec<_> = (0..12).map(|n| format!("nonterm_{n}")).collect();

        let body: Vec<_> = (0..12)
            .map(|n| Node::Nonterminal {
                span,
                name: &strings[n],
            })
            .collect();

        let root = Node::List { span, body };
        let tree = AsTree::new(&root);

        insta::assert_snapshot!(tree, @r"
        List [4294967294:0..4294967294:2]
        └─00: Nonterminal [4294967294:0..4294967294:2]
          │   └─ nonterm_0
          01: Nonterminal [4294967294:0..4294967294:2]
          │   └─ nonterm_1
          02: Nonterminal [4294967294:0..4294967294:2]
          │   └─ nonterm_2
          03: Nonterminal [4294967294:0..4294967294:2]
          │   └─ nonterm_3
          04: Nonterminal [4294967294:0..4294967294:2]
          │   └─ nonterm_4
          05: Nonterminal [4294967294:0..4294967294:2]
          │   └─ nonterm_5
          06: Nonterminal [4294967294:0..4294967294:2]
          │   └─ nonterm_6
          07: Nonterminal [4294967294:0..4294967294:2]
          │   └─ nonterm_7
          08: Nonterminal [4294967294:0..4294967294:2]
          │   └─ nonterm_8
          09: Nonterminal [4294967294:0..4294967294:2]
          │   └─ nonterm_9
          10: Nonterminal [4294967294:0..4294967294:2]
          │   └─ nonterm_10
          11: Nonterminal [4294967294:0..4294967294:2]
              └─ nonterm_11
        ");
    }
}

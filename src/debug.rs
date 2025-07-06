#![warn(warnings)]
use std::iter::once;

use display_tree::{AsTree, DisplayTree, Style};

use crate::{
    Rule,
    nodes::{NodeKind, NodePayload},
};

const EMPTY_STRING: &str = "";

impl DisplayTree for NodePayload<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter, style: Style) -> std::fmt::Result {
        let indentation = style.indentation as usize - 1;
        let horizontal_bar = format!("{:indentation$}", style.char_set.horizontal);

        if NodeKind::from(self) != NodeKind::Rule {
            let name: &str = NodeKind::from(self).into();
            writeln!(f, "{}", style.leaf_style.apply(name))?;
        }

        let spacer = format!(" {EMPTY_STRING:indentation$}");

        match self {
            NodePayload::Terminal(s) => write!(
                f,
                "{}",
                style.branch_style.apply(&format!(
                    "{}{horizontal_bar} '{}'",
                    style.char_set.end_connector,
                    s.escape_debug()
                ))
            )?,
            NodePayload::Regex(s) | NodePayload::Nonterminal(s) => write!(
                f,
                "{}",
                style.branch_style.apply(&format!(
                    "{}{horizontal_bar} {}",
                    style.char_set.end_connector, s
                ))
            )?,
            NodePayload::UnparsedOperator(unparsed_operator) => {
                let op: &str = unparsed_operator.into();
                write!(
                    f,
                    "{}",
                    style.branch_style.apply(&format!(
                        "{}{horizontal_bar} {}",
                        style.char_set.end_connector, op
                    ))
                )?;
            }

            NodePayload::Choice(nodes)
            | NodePayload::Optional(nodes)
            | NodePayload::Repeated(nodes)
            | NodePayload::List(nodes) => {
                let vec_output = fmt_vec(nodes, style);
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
            }
            NodePayload::Rule(rule) => write!(f, "{}", AsTree::new(rule))?,
        }
        Ok(())
    }
}

impl DisplayTree for Rule<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter, style: Style) -> std::fmt::Result {
        let indentation = style.indentation as usize - 1;
        let horizontal_bar = format!("{:indentation$}", style.char_set.horizontal);

        let spacer = format!(" {EMPTY_STRING:indentation$}");

        writeln!(f, "{}", style.leaf_style.apply("Rule"))?;
        writeln!(
            f,
            "{1}{horizontal_bar}name: {0}",
            &self.name, style.char_set.connector
        )?;
        let vec_output = fmt_vec(&self.body, style);

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
}

fn fmt_vec<T: DisplayTree>(v: &[T], style: Style) -> impl Iterator<Item = String> + '_ {
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
    use crate::{nodes::Node, token_data::Span};
    #[test]
    fn one_level_test() {
        let span = Span { start: 0, end: 0 };
        let strings: Vec<_> = (0..3).map(|n| format!("nonterm_{n}")).collect();

        let body: Vec<_> = (0..3)
            .map(|n| Node {
                span,
                payload: NodePayload::Nonterminal(&strings[n]),
            })
            .collect();

        let n = NodePayload::Rule(Rule { name: "name", body });
        let tree = AsTree::new(&n);

        insta::assert_snapshot!(tree, @r"
        Rule
        ├─name: name
        └─0: Nonterminal [0..0]
          │  └─ nonterm_0
          1: Nonterminal [0..0]
          │  └─ nonterm_1
          2: Nonterminal [0..0]
             └─ nonterm_2
        ");
    }

    #[test]
    fn long_list_test() {
        let span = Span { start: 0, end: 0 };
        let strings: Vec<_> = (0..12).map(|n| format!("nonterm_{n}")).collect();

        let vec: Vec<_> = (0..12)
            .map(|n| Node {
                span,
                payload: NodePayload::Nonterminal(&strings[n]),
            })
            .collect();

        let root = NodePayload::List(vec);
        let tree = AsTree::new(&root);

        insta::assert_snapshot!(tree, @r"
        List
        └─00: Nonterminal [0..0]
          │   └─ nonterm_0
          01: Nonterminal [0..0]
          │   └─ nonterm_1
          02: Nonterminal [0..0]
          │   └─ nonterm_2
          03: Nonterminal [0..0]
          │   └─ nonterm_3
          04: Nonterminal [0..0]
          │   └─ nonterm_4
          05: Nonterminal [0..0]
          │   └─ nonterm_5
          06: Nonterminal [0..0]
          │   └─ nonterm_6
          07: Nonterminal [0..0]
          │   └─ nonterm_7
          08: Nonterminal [0..0]
          │   └─ nonterm_8
          09: Nonterminal [0..0]
          │   └─ nonterm_9
          10: Nonterminal [0..0]
          │   └─ nonterm_10
          11: Nonterminal [0..0]
              └─ nonterm_11
        ");
    }
}

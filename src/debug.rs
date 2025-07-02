#![warn(warnings)]
use std::fmt::Display;

use display_tree::{AsTree, DisplayTree};

use crate::container::MyVec;

impl<T: DisplayTree> DisplayTree for MyVec<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter, style: display_tree::Style) -> std::fmt::Result {
        if self.is_empty() {
            return Ok(());
        }

        let max_index = self.len() - 1;
        let num_width = format!("{max_index}",).len();

        let vertical = style
            .branch_style
            .apply(&style.char_set.vertical.to_string());

        let iter = (0..1).flat_map(|_| self.iter()).enumerate();

        for (n, item) in iter {
            let continued_vertical = if n < max_index { &vertical } else { " " };

            let indent = format!(" {:num_width$}", "");

            write!(f, "{n:<0num_width$}: ")?;
            let tree = AsTree::with_style(item, style).to_string();
            for (line_num, line) in tree.lines().enumerate() {
                let line = style.leaf_style.apply(line);
                if line_num > 0 {
                    writeln!(f, "{continued_vertical}{indent}{line}")?;
                } else {
                    writeln!(f, "{line}")?;
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{nodes::Node, token_data::Span};
    #[test]
    fn one_level_test() {
        let span = Span { start: 0, end: 0 };
        let strings: Vec<_> = (0..3).map(|n| format!("nonterm_{n}")).collect();

        let vec: Vec<_> = (0..3)
            .map(|n| Node {
                span,
                payload: crate::nodes::NodePayload::Nonterminal(&strings[n]),
            })
            .collect();

        let tree = AsTree::new(&vec);
        //println!("{tree}");
        insta::assert_snapshot!(tree, @r"
        0: Nonterminal [0..0]
        │  └── nonterm_0
        1: Nonterminal [0..0]
        │  └── nonterm_1
        2: Nonterminal [0..0]
           └── nonterm_2
        ");
    }

    #[test]
    fn long_list_test() {
        let span = Span { start: 0, end: 0 };
        let strings: Vec<_> = (0..12).map(|n| format!("nonterm_{n}")).collect();

        let vec: Vec<_> = (0..12)
            .map(|n| Node {
                span,
                payload: crate::nodes::NodePayload::Nonterminal(&strings[n]),
            })
            .collect();

        let tree = AsTree::new(&vec);
        //println!("{tree}");

        insta::assert_snapshot!(tree, @r"
        00: Nonterminal [0..0]
        │   └── nonterm_0
        01: Nonterminal [0..0]
        │   └── nonterm_1
        02: Nonterminal [0..0]
        │   └── nonterm_2
        03: Nonterminal [0..0]
        │   └── nonterm_3
        04: Nonterminal [0..0]
        │   └── nonterm_4
        05: Nonterminal [0..0]
        │   └── nonterm_5
        06: Nonterminal [0..0]
        │   └── nonterm_6
        07: Nonterminal [0..0]
        │   └── nonterm_7
        08: Nonterminal [0..0]
        │   └── nonterm_8
        09: Nonterminal [0..0]
        │   └── nonterm_9
        10: Nonterminal [0..0]
        │   └── nonterm_10
        11: Nonterminal [0..0]
            └── nonterm_11
        ");
    }
}

impl<T: DisplayTree> Display for MyVec<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", AsTree::new(self))
    }
}

//! [![github]](https://github.com/ejmount/ebnf-toolkit)&ensp;[![crates-io]](https://crates.io/crates/ebnf-toolkit)&ensp;[![docs-rs]](crate)
//!
//! [github]: https://img.shields.io/github/v/release/ejmount/ebnf-toolkit?logo=github
//! [crates-io]: https://img.shields.io/crates/v/ebnf-toolkit
//! [docs-rs]: https://img.shields.io/docsrs/enbf-toolkit?logo=docsdotrs
//!
//!
//! The goal of `ebnf-toolkit` is manipulating context-free grammars written in various dialects of [Extended Backus–Naur form](https://en.wikipedia.org/wiki/Extended_Backus%E2%80%93Naur_form). While this includes parsing the grammar rules and making determinations about the grammar's properties, at this stage it does *not* include parsing input against an arbitrary grammar, as this is a very involved task.
//!
//! The toolkit's functionality is built from three types:
//! * [`Expr`] - an EBNF syntax node
//! * [`Rule`] - a production rule, associating the name of a nonterminal with a body consisting a sequence of `Expr`.
//! * [`Grammar`] - a set of `Rule`s. Using [`Grammar::new`] to construct a `Grammar` from a `&str` is likely your first port of call in using this crate.
//!
//! Parse failures from any of these types will produce an [`EbnfError`]. The type documentation has a breakdown of possible error conditions, but the value can be passed to `Display` to produce a human-readable report of what went wrong. For instance, attempting to parse `rule = (?;` via [`Rule::new`] will result in:
//!
//! ```plain
//!Error:
//!   ╭─[ <input>:1:1 ]
//!   │
//! 1 │ rule = (?;
//!   │        ┬┬┬
//!   │        ╰──── Possible unclosed bracket
//!   │         ││
//!   │         ╰─── Could not apply to preceding term
//!   │          │
//!   │          ╰── Rule ending here did not parse successfully
//!   │
//!   │ Note: The parse stack looked like this (most recent on top):
//!   │       └─0: UnparsedOperator [1:9..1:10]
//!   │         │  └─ Terminator
//!   │         1: UnparsedOperator [1:8..1:9]
//!   │         │  └─ Optional
//!   │         2: UnparsedOperator [1:7..1:8]
//!   │         │  └─ OpenedGroup
//!   │         3: UnparsedOperator [1:5..1:6]
//!   │         │  └─ Equals
//!   │         4: Nonterminal [1:0..1:4]
//!   │            └─ Rule
//! ──╯
//!```
//!
//! ## Syntax
//!
//! This crate's syntax closely follows that of [Kyle Lin's crate](https://github.com/ChAoSUnItY/ebnf) and by extension [instaparse](https://github.com/Engelberg/instaparse). A `Rule` and by extension a `Grammar` is built up by building the tree of `Expr` nodes from the input. The notation for each type of node is detailed in the table below. All text is assumed to be general UTF-8 except where specified.
//!
//! The string represention of an `Expr` can be retrieved by passing it through `Display`. Some nodes have multiple syntax options - they are parsed identically, although a node will not track which syntax was used to create and instead always uses a specific one. (That is, `Expr::new(expr.to_string())` is an elaborate no-op, but `expr.to_string()` is only one of potentially many strings that will result in the same `expr` value when parsed.)
//!
//! |Node|Syntax|Alternative|Notes|
//! |-|-|-|-|
//! |[`Literal`](`Expr::Literal`)| Any text except newlines between single or double quotes, e.g. `"hello"` || Quote marks can be escaped with a leading `\` - this is currently the only escape sequence processing|
//! |[`Nonterminal`](`Expr::Nonterminal`)| One or more letters, numbers or underscores || Yes, `_` and `42` are valid nonterminal names |
//! |[`Regex`](`Expr::Regex`)| `/regular expression/`| `#'regular expression'` | As defined by [regex](https://docs.rs/regex/latest/regex/), escapes within the regex are processed per that crate|
//! |[`Optional`](`Expr::Optional`)| `x?` | `[x]` ||
//! |[`Choice`](`Expr::Choice`)| `x\|y` | `x / y` | Both notations are infix |
//! |[`Repetition`](`Expr::Repetition`)| `x*` *or* `{x}` | `x+` | Either of the first two notations denotes zero-or-more - `x+` is specifically one-or-more|
//! |[`Group`](`Expr::Group`)| `(x...)` | | (This is unlikely to appear directly in output, see below)|
//! |[`Rule`](`Expr::Rule`)| `name = x...;` | | Any number of nodes may follow the `=` - terminating semicolon is mandatory|
//!
//! Concatenation in the body of a rule or within brackets (including the bracket notations for `Repetition` and `Optional`) may optionally use `,` but no separator is required, i.e. `(xy)` and `(x,y)` are equivalent. The expressions that can be written with brackets can contain any number of child nodes. (e.g. `[xyz]` is equivalent to `(xyz)?`, see below)
//!
//! ## Reductions
//!
//! In order to simplify making the `Display` string representation round-trip correctly, after a syntax tree is produced from the input string, it is then reduced to an equivalent but smaller tree by applying several rules:
//! * a series of consecutive choices, `a|b|c|d|...` is transformed into a single *n*-ary [`Choice`](`Expr::Choice`) node, `Choice { body: [a,b,c,d, ..], ..}` rather than a binary tree
//! * A `Group`, `Optional` or `Repetition` node `E` that contains a single `Group` child node is simplified by removing the intermediate node and placing its children as `E`'s direct children.
#![forbid(unsafe_code)]
#![warn(explicit_outlives_requirements)]
#![warn(missing_debug_implementations)]
#![warn(clippy::pedantic)]
#![warn(missing_copy_implementations)]
#![warn(redundant_lifetimes)]
#![warn(missing_docs)]
#![warn(unreachable_pub)]
#![warn(unused_crate_dependencies)]
#![warn(unused_qualifications)]
#![warn(unused)]
#![allow(clippy::must_use_candidate, reason = "Fires too often")]

mod debug;
mod error;
mod expr;
mod parser;
mod proptesting;
mod rule;
mod simplification;
mod token_data;

pub use crate::{
    error::{EbnfError, FailureReason},
    expr::Expr,
    rule::{Grammar, Rule},
    token_data::Span,
};

use crate::{
    parser::LrStack,
    simplification::simplify_node,
    token_data::{Token, TokenPayload},
};

fn parse_rules_from_tokens<'a>(
    input: &'a str,
    input_tokens: &mut &[Token<'a>],
) -> Result<Vec<Rule<'a>>, EbnfError<'a>> {
    let mut outputs = vec![];
    let mut stack = LrStack::new();

    let num_tokens = input_tokens.len();

    let Some(first_token) = input_tokens.split_off_first() else {
        return Err(EbnfError::EmptyInput);
    };
    stack.push_token(*first_token);

    let mut end_of_rule_expected = None;

    for n in 0..num_tokens {
        // There needs to be exactly one round of reducing where there is no remaining input, because the final token may be part of a reduction.
        // So this loop is half a cycle off of the intuitive shift-reduce order

        stack.reduce_until_shift_needed();

        if Some(n) == end_of_rule_expected && !matches!(stack.peek_node(), Some(Expr::Rule { .. }))
        {
            let offset = stack.peek_node().unwrap().span().start();
            return Err({
                EbnfError::ParseError {
                    input,
                    offset,
                    reason: Some(FailureReason::TerminatorNotEndingRule(
                        stack.into_parse_stack(),
                    )),
                }
            });
        }

        if let Some(Expr::Rule { .. }) = stack.peek_node() {
            let mut rule_node = stack.pop_node().unwrap();
            simplify_node(&mut rule_node);
            let Expr::Rule { rule, .. } = rule_node else {
                unreachable!()
            };
            outputs.push(rule);
        }

        if let Some(new_token) = input_tokens.split_off_first() {
            if TokenPayload::Termination == new_token.payload {
                end_of_rule_expected = Some(n + 1);
            }
            stack.push_token(*new_token);
        }
    }
    if stack.get(..).unwrap().is_empty() {
        Ok(outputs)
    } else {
        Err({
            EbnfError::ParseError {
                input,
                offset: input.len(),
                reason: Some(FailureReason::ExhaustedInput(stack.into_parse_stack())),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use display_tree::format_tree;

    use crate::{Expr, Rule};

    #[test]
    fn basic_success() {
        let src =
            "message       ::= ['@' tags SPACE] [':' source SPACE ] command [parameters] crlf;";

        let parse = Rule::new(src).unwrap_or_else(|e| panic!("{e}"));

        insta::assert_compact_debug_snapshot!(parse, @r#"Rule { name: "message", body: [Optional { span: Span { start: 19, end: 33, line_offset_start: (1, 19), line_offset_end: (1, 33) }, body: [Literal { span: Span { start: 19, end: 22, line_offset_start: (1, 19), line_offset_end: (1, 22) }, str: "@" }, Nonterminal { span: Span { start: 23, end: 27, line_offset_start: (1, 23), line_offset_end: (1, 27) }, name: "tags" }, Nonterminal { span: Span { start: 28, end: 33, line_offset_start: (1, 28), line_offset_end: (1, 33) }, name: "SPACE" }] }, Optional { span: Span { start: 36, end: 52, line_offset_start: (1, 36), line_offset_end: (1, 52) }, body: [Literal { span: Span { start: 36, end: 39, line_offset_start: (1, 36), line_offset_end: (1, 39) }, str: ":" }, Nonterminal { span: Span { start: 40, end: 46, line_offset_start: (1, 40), line_offset_end: (1, 46) }, name: "source" }, Nonterminal { span: Span { start: 47, end: 52, line_offset_start: (1, 47), line_offset_end: (1, 52) }, name: "SPACE" }] }, Nonterminal { span: Span { start: 55, end: 62, line_offset_start: (1, 55), line_offset_end: (1, 62) }, name: "command" }, Optional { span: Span { start: 64, end: 74, line_offset_start: (1, 64), line_offset_end: (1, 74) }, body: [Nonterminal { span: Span { start: 64, end: 74, line_offset_start: (1, 64), line_offset_end: (1, 74) }, name: "parameters" }] }, Nonterminal { span: Span { start: 76, end: 80, line_offset_start: (1, 76), line_offset_end: (1, 80) }, name: "crlf" }] }"#);
    }

    #[test]
    fn basic_span_check() {
        let src = "message       ::= hello;";

        let parse = Rule::new(src).unwrap_or_else(|e| panic!("{e}"));
        let node = parse.body.first().unwrap();
        let s = node.span();
        assert_eq!(&src[s.range()], "hello");
    }

    #[test]
    fn flatten_success() {
        let src = "A | (B | C) | D | E | F";

        let parse = Expr::new(src).unwrap_or_else(|e| panic!("{e}"));

        let tree = format_tree!(parse);
        insta::assert_snapshot!(tree, @r"
        Choice [1:0..1:23]
        └─0: Nonterminal [1:0..1:1]
          │  └─ A
          1: Nonterminal [1:5..1:6]
          │  └─ B
          2: Nonterminal [1:9..1:10]
          │  └─ C
          3: Nonterminal [1:14..1:15]
          │  └─ D
          4: Nonterminal [1:18..1:19]
          │  └─ E
          5: Nonterminal [1:22..1:23]
             └─ F
        ");
    }

    #[test]
    fn op_parse_fail() {
        let src = ";";
        Expr::new(src).unwrap_err();
    }

    #[test]
    #[should_panic]
    fn bracket_parse_fail() {
        let src = "{}";
        Expr::new(src).unwrap_or_else(|e| panic!("{e}"));
    }
}

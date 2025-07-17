#![forbid(unsafe_code)]
#![warn(explicit_outlives_requirements)]
#![warn(missing_debug_implementations)]
#![warn(clippy::pedantic)]
#![warn(missing_copy_implementations)]
#![warn(redundant_lifetimes)]
//#![warn(missing_docs)]
#![warn(unreachable_pub)]
#![warn(unused_crate_dependencies)]
#![warn(unused_qualifications)]
#![allow(clippy::must_use_candidate)]

mod debug;
mod error;
mod nodes;
mod parser;
mod rule;
mod simplification;
mod token_data;

pub use error::{EbnfError, FailureReason};
pub use nodes::Node;
pub use rule::{Grammar, Rule};
pub use token_data::Span;

use crate::{
    parser::LrStack,
    simplification::tidy_up_rule,
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

        stack.reduce_until_shift_needed().map_err(|(stack, n)| {
            EbnfError::from_parse_error(
                input,
                stack.clone(),
                n.span().start(),
                Some(n.span().end()),
                Some(FailureReason::InvalidRuleStart(n.clone())),
            )
        })?;

        #[cfg(debug_assertions)]
        if Some(n) == end_of_rule_expected && !matches!(stack.peek_node(), Some(Node::Rule { .. }))
        {
            let offset = stack.peek_node().unwrap().span().start();
            return Err(EbnfError::from_parse_error(
                input,
                stack.clone(),
                offset,
                None,
                Some(FailureReason::TerminatorNotEndingRule(
                    stack.get(..).unwrap().to_vec(),
                )),
            ));
        }

        if let Some(Node::Rule { .. }) = stack.peek_node() {
            let Node::Rule { rule, .. } = stack.pop_node().unwrap() else {
                unreachable!()
            };
            outputs.push(tidy_up_rule(rule));
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
        let reason = FailureReason::ExhaustedInput(stack.get(..).unwrap().to_vec());
        Err(EbnfError::from_parse_error(
            input,
            stack,
            input.len(),
            None,
            Some(reason),
        ))
    }
}

#[cfg(test)]
mod tests {
    use display_tree::format_tree;

    use crate::Rule;

    #[test]
    fn basic_success() {
        let src =
            "message       ::= ['@' tags SPACE] [':' source SPACE ] command [parameters] crlf;";

        let parse = Rule::new(src).unwrap_or_else(|e| panic!("{e}"));

        //let tree = format_tree!(parse);
        insta::assert_compact_debug_snapshot!(parse, @r#"Rule { name: "message", body: [Optional { span: Span { start: 18, end: 34, line_offset_start: (1, 18), line_offset_end: (1, 34) }, body: [Terminal { span: Span { start: 19, end: 22, line_offset_start: (1, 19), line_offset_end: (1, 22) }, str: "@" }, Nonterminal { span: Span { start: 23, end: 27, line_offset_start: (1, 23), line_offset_end: (1, 27) }, name: "tags" }, Nonterminal { span: Span { start: 28, end: 33, line_offset_start: (1, 28), line_offset_end: (1, 33) }, name: "SPACE" }] }, Optional { span: Span { start: 35, end: 54, line_offset_start: (1, 35), line_offset_end: (1, 54) }, body: [Terminal { span: Span { start: 36, end: 39, line_offset_start: (1, 36), line_offset_end: (1, 39) }, str: ":" }, Nonterminal { span: Span { start: 40, end: 46, line_offset_start: (1, 40), line_offset_end: (1, 46) }, name: "source" }, Nonterminal { span: Span { start: 47, end: 52, line_offset_start: (1, 47), line_offset_end: (1, 52) }, name: "SPACE" }] }, Nonterminal { span: Span { start: 55, end: 62, line_offset_start: (1, 55), line_offset_end: (1, 62) }, name: "command" }, Optional { span: Span { start: 63, end: 75, line_offset_start: (1, 63), line_offset_end: (1, 75) }, body: [Nonterminal { span: Span { start: 64, end: 74, line_offset_start: (1, 64), line_offset_end: (1, 74) }, name: "parameters" }] }, Nonterminal { span: Span { start: 76, end: 80, line_offset_start: (1, 76), line_offset_end: (1, 80) }, name: "crlf" }] }"#);
    }

    #[test]
    fn flatten_success() {
        let src = "success = A | (B | C) | D | E | F;";

        let parse = Rule::new(src).unwrap_or_else(|e| panic!("{e}"));

        let tree = format_tree!(parse);
        insta::assert_snapshot!(tree, @r"
        Rule
        ├─name: success
        └─0: Choice [1:10..1:33]
             └─0: Nonterminal [1:10..1:11]
               │  └─ A
               1: Group [1:14..1:21]
               │  └─0: Choice [1:15..1:20]
               │       └─0: Nonterminal [1:15..1:16]
               │         │  └─ B
               │         1: Nonterminal [1:19..1:20]
               │            └─ C
               2: Nonterminal [1:24..1:25]
               │  └─ D
               3: Nonterminal [1:28..1:29]
               │  └─ E
               4: Nonterminal [1:32..1:33]
                  └─ F
        ");
    }
}

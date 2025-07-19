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
            let mut rule_node = stack.pop_node().unwrap();
            simplify_node(&mut rule_node);
            let Node::Rule { rule, .. } = rule_node else {
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

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

mod debug;
mod error;

mod nodes;
mod parser;
mod token_data;

use std::collections::HashMap;

pub use nodes::{Node, Rule};
pub use token_data::Span;

use crate::{
    error::{EbnfError, FailureReason},
    nodes::NodePayload,
    parser::LrStack,
    token_data::{Token, TokenPayload, tokenize},
};

pub fn parse_rule(input: &str) -> Result<Rule<'_>, EbnfError> {
    let tokens = tokenize(input)?;

    let mut tokens_buffer = &tokens[..];
    parse_rule_from_tokens(input, &mut tokens_buffer)
}

fn parse_rule_from_tokens<'a>(
    input: &'a str,
    input_tokens: &mut &[Token<'a>],
) -> Result<Rule<'a>, EbnfError<'a>> {
    let mut stack = LrStack::new();

    let num_tokens = input_tokens.len();

    let Some(first_token) = input_tokens.split_off_first() else {
        return Err(EbnfError::EmptyInput);
    };
    stack.push_token(*first_token);

    let mut end_of_rule_expected = None;

    for n in 0..num_tokens {
        // During the shift-reduce cycle, the lookahead is used to ensure reductions are greedy, but is never incorporated into the reduced node itself.
        // This means there needs to be exactly one round of reducing where there is no lookhead token, because the final token may be part of a reduction. (But since every round repeatedly reduces until a shift is needed, a second empty lookahead would be a waste)

        let lookahead = input_tokens.split_off_first();

        if matches!(
            lookahead,
            Some(Token {
                payload: TokenPayload::Termination,
                ..
            })
        ) {
            end_of_rule_expected = Some(n + 1);
        }

        stack.reduce_until_shift_needed(lookahead);

        if Some(n) == end_of_rule_expected
            && !matches!(
                stack.peek_node(),
                Some(Node {
                    payload: NodePayload::Rule(_),
                    ..
                })
            )
        {
            let tail = &input[stack.peek_node().unwrap().span.start..];
            return Err(EbnfError::from_parse_error(
                input,
                stack,
                n,
                Some(FailureReason::TerminatorNotEndingRule(tail)),
            ));
        }

        let stack_top = stack.peek_node();
        if let Some(Node {
            payload: NodePayload::Rule(_),
            ..
        }) = stack_top
        {
            let NodePayload::Rule(r) = stack.pop_node().unwrap().payload else {
                unreachable!()
            };
            return Ok(r);
        }

        if let Some(head) = lookahead {
            stack.push_token(*head);
        }
    }
    if stack.get(..).unwrap().is_empty() {
        unreachable!()
    } else {
        Err(EbnfError::from_parse_error(
            input,
            stack,
            input.len(),
            Some(FailureReason::ExhaustedInput),
        ))
    }
}

pub fn parse_grammar(input: &str) -> Result<HashMap<&str, Rule<'_>>, EbnfError<'_>> {
    let mut output = HashMap::new();
    let tokens = tokenize(input)?;

    let mut tokens_buffer = &tokens[..];
    while !tokens_buffer.is_empty() {
        let rule = parse_rule_from_tokens(input, &mut tokens_buffer)?;
        dbg!(tokens_buffer.len(), rule.name);
        output.insert(rule.name, rule);
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use crate::{parse_grammar, parse_rule};

    #[test]
    fn basic_success() {
        let src =
            "message       ::= ['@' tags SPACE] [':' source SPACE ] command [parameters] crlf;";

        let parse = parse_rule(src).unwrap_or_else(|e| panic!("{e}"));

        //let tree = format_tree!(parse);
        insta::assert_compact_debug_snapshot!(parse, @r#"Rule { name: "message", body: [Node { span: Span { start: 18, end: 34 }, payload: Optional([Node { span: Span { start: 19, end: 22 }, payload: Terminal("@") }, Node { span: Span { start: 23, end: 27 }, payload: Nonterminal("tags") }, Node { span: Span { start: 28, end: 33 }, payload: Nonterminal("SPACE") }]) }, Node { span: Span { start: 35, end: 54 }, payload: Optional([Node { span: Span { start: 36, end: 39 }, payload: Terminal(":") }, Node { span: Span { start: 40, end: 46 }, payload: Nonterminal("source") }, Node { span: Span { start: 47, end: 52 }, payload: Nonterminal("SPACE") }]) }, Node { span: Span { start: 55, end: 62 }, payload: Nonterminal("command") }, Node { span: Span { start: 63, end: 75 }, payload: Optional([Node { span: Span { start: 64, end: 74 }, payload: Nonterminal("parameters") }]) }, Node { span: Span { start: 76, end: 80 }, payload: Nonterminal("crlf") }] }"#);
    }

    #[test]
    fn file_parse() {
        let file = include_str!(r"..\tests\irc.ebnf");

        let grammar = parse_grammar(file).unwrap_or_else(|e| panic!("{e}"));

        insta::assert_debug_snapshot!(grammar);
    }
}

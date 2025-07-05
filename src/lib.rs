#![warn(explicit_outlives_requirements)]
#![warn(missing_debug_implementations)]
#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]
#![warn(missing_copy_implementations)]
#![warn(redundant_lifetimes)]
#![warn(missing_docs)]
#![warn(unreachable_pub)]
#![warn(unused_crate_dependencies)]
#![warn(unused_qualifications)]
#![allow(clippy::enum_glob_use)]

mod debug;
mod error;

mod nodes;
mod parser;
mod token_data;

use std::collections::HashMap;

pub use nodes::{Node, Rule};
pub use token_data::Span;

use crate::{
    error::EbnfError,
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

    for _ in 0..num_tokens {
        // During the shift-reduce cycle, the lookahead is used to ensure reductions are greedy, but is never incorporated into the reduced node itself.
        // This means there needs to be exactly one round of reducing where there is no lookhead token, because the final token may be part of a reduction. (But since every round repeatedly reduces until a shift is needed, a second empty lookahead would be a waste)

        let lookahead = input_tokens.split_off_first();

        let end_of_rule_expected = matches!(
            lookahead,
            Some(Token {
                payload: TokenPayload::Termination,
                ..
            })
        );

        stack.reduce_until_shift_needed(lookahead);

        // if end_of_rule_expected
        //     && !matches!(
        //         stack.peek_node(),
        //         Some(Node {
        //             payload: NodePayload::Rule(_),
        //             ..
        //         })
        //     )
        // {
        //     return Err(EbnfError::from_parse_error(input, stack));
        // }

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
        Err(EbnfError::from_parse_error(input, stack))
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
    use display_tree::format_tree;

    use crate::{parse_grammar, parse_rule};

    #[test]
    fn basic_success() {
        let src =
            "message       ::= ['@' tags SPACE] [':' source SPACE ] command [parameters] crlf;";

        let parse = parse_rule(src).unwrap();

        let tree = format_tree!(parse);
        println!("{tree}");
    }

    #[test]
    fn file_parse() {
        let file = include_str!(r"..\tests\irc.ebnf");

        let grammar = parse_grammar(file).unwrap_or_else(|e| panic!("{e}"));

        insta::assert_debug_snapshot!(grammar);
    }
}

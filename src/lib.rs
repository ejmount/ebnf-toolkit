#![warn(explicit_outlives_requirements)]
#![warn(missing_debug_implementations)]
#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]
#![warn(missing_copy_implementations)]
#![warn(redundant_lifetimes)]
//#![warn(missing_docs)]
#![warn(unreachable_pub)]
#![warn(unused_crate_dependencies)]
#![warn(unused_qualifications)]
#![allow(clippy::enum_glob_use)]

use winnow::LocatingSlice;

use crate::{error::EbnfError, nodes::Rule, parser::file_reduce};

mod container;
mod debug;
mod error;
mod lexing;
mod logos_lexer;
mod nodes;
mod parser;
mod token_data;

pub(crate) type RawInput<'a> = LocatingSlice<&'a str>;

// #[test]
// pub fn old_test() {
//     let text = "foo = ('hello'?)?;";
//     let grammar = ebnf::get_grammar(text).unwrap();
//     panic!("{grammar:#?}");
// }

pub fn parse_rule(_input: &str) -> Result<Rule<'_>, EbnfError<'_, '_>> {
    file_reduce();
    todo!()
}

#[cfg(test)]
mod tests {

    // #[test]
    // fn success() {
    //     let src =
    //         "message       ::= ['@' tags SPACE] [':' source SPACE ] command [parameters] crlf;";

    //     let ls = LocatingSlice::new(src);

    //     let tokens = tokenize(ls).unwrap();
    //     let mut slice = TokenSlice::new(&tokens);

    //     let parse = Rule::parser(&mut slice).unwrap();

    //     let tree = format_tree!(parse);
    //     println!("{tree}");
    // }
}

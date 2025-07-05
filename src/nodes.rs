use crate::token_data::Span;
use display_tree::{AsTree, DisplayTree};
use std::fmt::Display;

use strum::{EnumCount, EnumDiscriminants, EnumProperty, IntoStaticStr, VariantNames};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule<'a> {
    pub name: &'a str,
    pub body: Vec<Node<'a>>,
}

// impl Display for Rule<'_> {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "{}", AsTree::new(self))
//     }
// }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node<'a> {
    pub(crate) span: Span,

    pub(crate) payload: NodePayload<'a>,
}

impl DisplayTree for Node<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter, style: display_tree::Style) -> std::fmt::Result {
        let payload = AsTree::with_style(&self.payload, style).to_string();
        let mut lines = payload.lines();
        let first_line = lines.next().unwrap();
        writeln!(f, "{first_line} {}", self.span)?;
        for line in lines {
            writeln!(f, "{line}")?;
        }
        Ok(())
    }
}

// impl Display for Node<'_> {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "{}", AsTree::new(self))
//     }
// }

#[derive(Debug, Clone, PartialEq, Eq, EnumProperty, EnumDiscriminants)]
#[strum_discriminants(name(NodeKind), derive(EnumCount, VariantNames, IntoStaticStr))]
pub(crate) enum NodePayload<'a> {
    Terminal(&'a str),
    Nonterminal(&'a str),
    Choice(Vec<Node<'a>>),
    Optional(Vec<Node<'a>>),
    Repeated(Vec<Node<'a>>),
    Regex(&'a str),
    List(Vec<Node<'a>>),
    UnparsedOperator(UnparsedOperator),
    Rule(Rule<'a>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, EnumProperty, IntoStaticStr)]
pub(crate) enum UnparsedOperator {
    #[strum(props(string = "("))]
    OpenedGroup,
    #[strum(props(string = ")"))]
    ClosedGroup,
    #[strum(props(string = "["))]
    OpenedSquare,
    #[strum(props(string = "]"))]
    ClosedSquare,
    #[strum(props(string = ";"))]
    Terminator,
    #[strum(props(string = "="))]
    Equals,
    #[strum(props(string = "|"))]
    Alternation,
    #[strum(props(string = "*"))]
    Kleene,
    #[strum(props(string = "?"))]
    Optional,
    #[strum(props(string = "+"))]
    Repeat,
}

impl Display for UnparsedOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl From<&'_ Node<'_>> for NodeKind {
    fn from(value: &Node) -> Self {
        Self::from(&value.payload)
    }
}

#[cfg(test)]
mod test {

    // #[test]
    // fn basic_parse() {
    //     let src = "foo ::= [(bar)(baz)];";
    //     let src = LocatingSlice::new(src);
    //     let tokens = tokenize(src).unwrap();
    //     let mut input = LexedInput::new(&tokens);
    //     let result = Rule::parser.parse_next(&mut input).unwrap();

    //     let tree = format_tree!(result);

    //     insta::assert_snapshot!(tree, @r#"
    //     Rule
    //     ├── name: Identifier [0..3]("foo")
    //     └── tree: Sequence [8..20]
    //         └── 0: Sequence [8..20]
    //                └── 0: Sequence [9..14]
    //                    │  └── 0: Nonterminal [10..13]
    //                    │         └── bar
    //                    1: Sequence [14..19]
    //                       └── 0: Nonterminal [15..18]
    //                              └── baz
    //     "#);
    //     //panic!();
    // }

    // #[test]
    // fn reporting_unexpected_token() {
    //     use crate::parser::InternalErrorType;
    //     const BAD_SRC: &str = "longlabel ::= ::=";
    //     let bad_input = LocatingSlice::new(BAD_SRC);
    //     let bad_tokens = tokenize(bad_input).unwrap();
    //     let bad_tokens = TokenSlice::new(&bad_tokens);
    //     let bad_result = Rule::parser.parse(bad_tokens);

    //     let err = bad_result.unwrap_err();
    //     let offset = err.offset();
    //     let ctx: Vec<_> = err.inner().context().collect();
    //     let tok_context = ctx.first().unwrap();

    //     let span = if let InternalErrorType::TokenError(token_error) = tok_context {
    //         token_error.found.unwrap().span
    //     } else {
    //         unimplemented!()
    //     };

    //     assert_compact_debug_snapshot!(tok_context, @"TokenError(TokenError { expected: TokenSet([Identifier, Alternation, Optional, String, OpeningGroup, ClosingGroup, OpeningSquare, ClosingSquare, Whitespace]), found: Some(Equals [14..17]) })");

    //     assert_eq!(&BAD_SRC[span.start..], "::=");
    // }
}

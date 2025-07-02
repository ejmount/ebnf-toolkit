use std::{fmt::Display, iter::once, ops::Range, slice::SliceIndex, str::FromStr};

use crate::{
    container::MyVec as Vec,
    error::{
        BracketingError::{self, UnexpectedClose},
        InternalErrorType, TokenError,
    },
    token_data::{LexedInput, Span, Token, TokenKind, TokenPayload, TokenSet, TokenStore},
};

use regex::{Match, Regex};
use strum::{
    EnumCount, EnumDiscriminants, EnumProperty, EnumString, IntoStaticStr, VariantArray,
    VariantNames,
};

#[derive(Debug, Clone, PartialEq, Eq, DisplayTree)]
pub struct Rule<'a> {
    #[field_label]
    pub name: Box<Node<'a>>,
    #[tree]
    #[field_label]
    pub body: Vec<Node<'a>>,
}

impl Display for Rule<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", AsTree::new(self))
    }
}

use display_tree::{AsTree, DisplayTree};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node<'a> {
    //#[node_label]
    pub(crate) span: Span,

    //#[tree]
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

impl Display for Node<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", AsTree::new(self))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, EnumProperty, EnumDiscriminants, DisplayTree)]
#[strum_discriminants(name(NodeKind), derive(EnumCount, VariantNames, IntoStaticStr))]
pub(crate) enum NodePayload<'a> {
    Terminal(&'a str),
    Nonterminal(&'a str),
    Choice(#[tree] Vec<Node<'a>>),
    Optional(#[tree] Vec<Node<'a>>),
    Repeated(#[tree] Vec<Node<'a>>),
    Regex(&'a str),
    List(#[tree] Vec<Node<'a>>),

    UnparsedToken(Token<'a>),
    Rule(Rule<'a>),
}

impl From<&'_ Node<'_>> for NodeKind {
    fn from(value: &Node) -> Self {
        Self::from(&value.payload)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LrStack<'a> {
    bracket_stack: Vec<(usize, Token<'a>)>,
    pub(crate) token_stack: Vec<Node<'a>>,
    pub(crate) token_pattern: String,
}

impl<'a> LrStack<'a> {
    pub(crate) fn new() -> LrStack<'a> {
        LrStack {
            bracket_stack: Vec::new(),
            token_stack: Vec::new(),
            token_pattern: String::from_str("").unwrap(),
        }
    }

    pub(crate) fn get<I: SliceIndex<[Node<'a>]>>(
        &self,
        index: I,
    ) -> Option<&<I as SliceIndex<[Node<'a>]>>::Output> {
        self.token_stack.get(index)
    }

    pub(crate) fn match_rule(&self, r: &Regex) -> Option<Range<usize>> {
        r.find(&self.token_pattern).as_ref().map(Match::range)
    }

    pub(crate) fn push_token(&mut self, t: Token<'a>) {
        use TokenPayload::*;
        let Token { payload, span } = t;
        let payload = match payload {
            Alternation | OpeningSquare | ClosingSquare | Equals | Termination | Kleene
            | OpeningGroup | ClosingGroup | Optional | Repeat => NodePayload::UnparsedToken(t),
            String(s) => NodePayload::Terminal(s),
            Identifier(s) => NodePayload::Nonterminal(s),
            Regex(s) => NodePayload::Regex(s),
            Whitespace(_) | Newline => unreachable!(),
        };
        self.push_node(Node { span, payload });
    }

    pub(crate) fn push_node(&mut self, n: Node<'a>) {
        let kind = if let NodePayload::UnparsedToken(t) = n.payload {
            t.payload.get_str("string").unwrap()
        } else {
            let nk = NodeKind::from(&n.payload);
            let name: &str = nk.into();
            &name[..1]
        };
        self.token_pattern.push_str(kind);
        self.token_stack.push(n);
    }

    fn pop_node(&mut self) -> Option<Node<'a>> {
        self.token_pattern.pop();
        self.token_stack.pop()
    }

    pub(crate) fn drop_many(&mut self, n: usize) {
        for _ in 0..n {
            self.pop_node();
        }
    }
}

#[cfg(test)]
mod test {
    use super::Rule;
    use display_tree::{AsTree, Color, Style, StyleBuilder, format_tree};
    use insta::assert_compact_debug_snapshot;
    use winnow::Parser;
    use winnow::{LocatingSlice, error::ParseError, stream::TokenSlice};

    use crate::container::MyVec as Vec;
    use crate::token_data::LexedInput;
    use crate::{error::TokenError, lexing::tokenize};

    #[test]
    fn basic_parse() {
        let src = "foo ::= [(bar)(baz)];";
        let src = LocatingSlice::new(src);
        let tokens = tokenize(src).unwrap();
        let mut input = LexedInput::new(&tokens);
        let result = Rule::parser.parse_next(&mut input).unwrap();

        let tree = format_tree!(result);

        insta::assert_snapshot!(tree, @r#"
        Rule
        ├── name: Identifier [0..3]("foo")
        └── tree: Sequence [8..20]
            └── 0: Sequence [8..20]
                   └── 0: Sequence [9..14]
                       │  └── 0: Nonterminal [10..13]
                       │         └── bar
                       1: Sequence [14..19]
                          └── 0: Nonterminal [15..18]
                                 └── baz
        "#);
        //panic!();
    }

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

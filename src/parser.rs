use crate::{
    Span,
    error::TokenContext,
    token_data::{LexedInput, Token, TokenKind, TokenStore, any_token},
};

use winnow::{
    Parser,
    combinator::{repeat, seq},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LexedRule<'a> {
    name: Token<'a>,
    tree: TokenStore<'a>,
}

const RULE_BODY_TOKENS: &[TokenKind] = &[
    TokenKind::Identifier,
    TokenKind::Alternation,
    TokenKind::Optional,
    TokenKind::String,
    TokenKind::OpeningGroup,
    TokenKind::ClosingGroup,
    TokenKind::OpeningSquare,
    TokenKind::ClosingSquare,
    TokenKind::Whitespace,
];

fn parse_rule<'a>() -> impl Parser<LexedInput<'a>, LexedRule<'a>, TokenContext> {
    use TokenKind::*;
    seq! {
       LexedRule {
           name: Identifier,
           _: Equals,
           tree: repeat(1.., any_token(RULE_BODY_TOKENS)),
           _: Termination,
       }
    }
}

#[allow(unused)]
fn parse_rule_body(_input: &mut LexedInput) -> Result<(), TokenContext> {
    Ok(())
}

#[allow(unused)]
pub struct Node {
    span: Span,
}

#[cfg(test)]
mod test {
    use super::parse_rule;
    use insta::assert_compact_debug_snapshot;
    use winnow::Parser;
    use winnow::{LocatingSlice, error::ParseError, stream::TokenSlice};

    use crate::{
        error::TokenError,
        lexing::{TokenKind, tokenize},
    };

    #[test]
    fn reporting_unexpected_token() {
        let bad_input = LocatingSlice::new("bad ::= ::=");
        let mut bad_tokens = tokenize(bad_input).unwrap();
        let bad_tokens = TokenSlice::new(&bad_tokens);
        let bad_result = parse_rule().parse(bad_tokens);

        let err = bad_result.unwrap_err();
        let ctx: Vec<_> = err.inner().context().collect();
        let tok_context = ctx.first().unwrap();

        assert_compact_debug_snapshot!(tok_context, @"TokenError { expected: TokenSet([Identifier, Alternation, Optional, String, OpeningGroup, ClosingGroup, OpeningSquare, ClosingSquare, Whitespace]), found: Some(Equals) }");
    }
}

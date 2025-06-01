use crate::{
    LexedInput, Span,
    error::{TokenContext, TokenError},
    token::{Token, TokenKind, TokenSet, any_tag},
};

use winnow::{
    Parser,
    combinator::{repeat, seq},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule<'a> {
    name: Token<'a>,
    tree: Vec<Token<'a>>,
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

fn parse_rule<'a>() -> impl Parser<LexedInput<'a>, Rule<'a>, TokenContext> {
    seq! {
       Rule {
           name: TokenKind::Identifier,
           _: repeat(0.., TokenKind::Whitespace).fold(|| (), |_,_| ()),
           _: TokenKind::Equals,
           tree: repeat(1.., any_tag(RULE_BODY_TOKENS).context(TokenError {
            expected: TokenSet::new(&[TokenKind::Alternation]),
            found: TokenKind::Alternation.into(),
        })),
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
    use insta::{assert_compact_debug_snapshot, assert_debug_snapshot, assert_snapshot};
    use winnow::{LocatingSlice, error::ParseError, stream::TokenSlice};

    use crate::{
        LexedInput,
        error::TokenError,
        token::{TokenKind, tokenize},
    };

    use super::parse_rule;
    use winnow::Parser;

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

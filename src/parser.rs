use crate::{
    LexedInput, Span,
    error::TokenContext,
    token::{Token, TokenKind, any_tag},
};

use winnow::{
    Parser,
    combinator::{repeat, seq},
    error::ContextError,
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

fn parse_rule<'a>() -> impl Parser<LexedInput<'a>, Rule<'a>, ContextError<TokenContext<'static>>> {
    seq! {
       Rule {
           name: TokenKind::Identifier,
           _: repeat(0.., TokenKind::Whitespace).fold(|| (), |_,_| ()),
           _: TokenKind::Equals,
           tree: repeat(1.., any_tag(RULE_BODY_TOKENS).context(TokenContext {
            expected: &[TokenKind::Alternation],
            found: TokenKind::Alternation.into(),
        })),
       }
    }
}

#[allow(unused)]
fn parse_rule_body<'a>(_input: &mut LexedInput) -> Result<(), ContextError<TokenContext<'a>>> {
    Ok(())
}

#[allow(unused)]
pub struct Node {
    span: Span,
}

#[cfg(test)]
mod test {
    use insta::{assert_debug_snapshot, assert_snapshot};
    use winnow::{LocatingSlice, error::ParseError, stream::TokenSlice};

    use crate::{
        LexedInput,
        error::TokenContext,
        token::{TokenKind, tokenize},
    };

    use super::parse_rule;
    use winnow::Parser;

    #[test]
    fn reporting_unexpected_token() {
        let bad_input = LocatingSlice::new("bad ::= ::=");
        let mut bad_tokens = tokenize(bad_input).unwrap();
        bad_tokens.retain(|t| TokenKind::from(t.payload()) != TokenKind::Whitespace);
        let bad_tokens = TokenSlice::new(&bad_tokens);
        let bad_result = parse_rule().parse(bad_tokens);

        let err = bad_result.unwrap_err();
        let ctx: Vec<_> = err.inner().context().collect();
        let tok_context = ctx.first().unwrap();

        assert_debug_snapshot!(tok_context, @r"
        TokenContext {
            expected: [
                Identifier,
                Alternation,
                Optional,
                String,
                OpeningGroup,
                ClosingGroup,
                OpeningSquare,
                ClosingSquare,
                Whitespace,
            ],
            found: Some(
                Equals,
            ),
        }
        ");
    }
}

#[cfg(test)]
use insta::assert_compact_debug_snapshot;
use logos::Logos;

use crate::{
    error::EbnfError,
    token_data::{Token, TokenPayload, TokenStore},
};

pub(crate) fn tokenize(input: &str) -> Result<TokenStore<'_>, EbnfError<'_, '_>> {
    let lexer = TokenPayload::lexer(input).spanned();

    let mut output = vec![];
    for (payload, s) in lexer {
        if let Ok(payload) = payload {
            let t = Token {
                span: s.into(),
                payload,
            };

            output.push(t);
        } else {
            //dbg!(output);
            return Err(EbnfError::LexError3(&input[s.start..]));
        }
    }
    Ok(TokenStore(output))
}

#[cfg(test)]
#[test]
fn basic_token_test() {
    let input = "message       ::= ['@' tags SPACE] [':' source SPACE ] command [parameters] crlf;";

    let tokens = tokenize(input).unwrap();

    assert_compact_debug_snapshot!(&tokens[..], @r#"[Identifier [0..7]("message"), Equals [14..17], OpeningSquare [18..19], String [19..22]("@"), Identifier [23..27]("tags"), Identifier [28..33]("SPACE"), ClosingSquare [33..34], OpeningSquare [35..36], String [36..39](":"), Identifier [40..46]("source"), Identifier [47..52]("SPACE"), ClosingSquare [53..54], Identifier [55..62]("command"), OpeningSquare [63..64], Identifier [64..74]("parameters"), ClosingSquare [74..75], Identifier [76..80]("crlf"), Termination [80..81]]"#);
}

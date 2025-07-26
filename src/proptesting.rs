#![cfg(test)]
use crate::Expr;
use crate::Rule;
use crate::simplification::simplify_node;
use crate::token_data::DUMMY_SPAN;
use display_tree::AsTree;
use proptest::prelude::*;
use proptest::prop_oneof;

const NUM_NAMES: usize = 128;

const NAMES: [&str; NUM_NAMES] = const {
    const NUM_LENGTH: usize = 4;
    const DIGITS: [u8; 10] = [b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9'];
    const PREFIX: &str = "string";

    const BYTE_BLOCKS: [[u8; PREFIX.len() + NUM_LENGTH]; 128] = {
        let mut orig = [[0; PREFIX.len() + NUM_LENGTH]; 128];
        let mut name_idx = 0;
        while name_idx < orig.len() {
            let leading = orig[name_idx]
                .first_chunk_mut::<{ PREFIX.len() }>()
                .unwrap();
            leading.copy_from_slice(PREFIX.as_bytes());
            let mut digit = NUM_LENGTH;
            while digit > 0 {
                let shift = name_idx / (10usize.pow((digit - 1) as u32));

                orig[name_idx][PREFIX.len() + NUM_LENGTH - digit] = DIGITS[shift % DIGITS.len()];

                digit -= 1;
            }
            name_idx += 1;
        }
        orig
    };

    let mut output = [""; NUM_NAMES];
    let mut a = 0;
    while a < output.len() {
        output[a] = match str::from_utf8(&BYTE_BLOCKS[a]) {
            Ok(s) => s,
            Err(_) => unreachable!(),
        };
        a += 1;
    }

    output
};

fn node_strategy() -> impl Strategy<Value = Expr<'static>> {
    let leaf = (0..NAMES.len() * 3).prop_map(|n| {
        let typ = n % 3;
        let n = n / 3;
        match typ {
            0 => Expr::Nonterminal {
                span: DUMMY_SPAN,
                name: NAMES[n],
            },
            1 => Expr::Literal {
                span: DUMMY_SPAN,
                str: NAMES[n],
            },
            2 => Expr::Regex {
                span: DUMMY_SPAN,
                pattern: NAMES[n],
            },
            _ => unreachable!(),
        }
    });

    leaf.prop_recursive(2, 10, 2, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 2).prop_map(|body| Expr::Choice {
                span: DUMMY_SPAN,
                body
            }),
            prop::collection::vec(inner.clone(), 2).prop_map(|body| Expr::Optional {
                span: DUMMY_SPAN,
                body
            }),
            (prop::collection::vec(inner.clone(), 2), any::<bool>()).prop_map(
                |(body, one_needed)| Expr::Repetition {
                    span: DUMMY_SPAN,
                    body,
                    one_needed,
                }
            ),
            prop::collection::vec(inner.clone(), 2).prop_map(|body| Expr::Group {
                span: DUMMY_SPAN,
                body
            }),
        ]
    })
}

#[test]
fn test_names() {
    assert_eq!(
        *NAMES.last().unwrap(),
        &format!("string{:04}", NAMES.len() - 1)
    );
}

#[test]
fn expr_parse_rule() {
    let src = "foo = bar;";
    let e = Expr::new(src).unwrap();
    assert!(matches!(e, Expr::Rule { .. }), "{} not a Rule", e);
}

proptest! {
    #[test]
    fn display_rule_roundtrip(mut n in node_strategy()) {
        simplify_node(&mut n);
        let string = format!("{n}");
        let rule = format!("rule = {string};");
        let Rule { mut body, .. } = Rule::new(&rule).unwrap_or_else(|e| panic!("{e}"));
        let mut actual = body.pop().unwrap();

        simplify_node(&mut actual);

        if n != actual {
            let actual_tree = AsTree::new(&actual);
            let n_tree = AsTree::new(&n);

            eprintln!("Got: {actual}\nExpected:{n}");
            eprintln!();
            eprintln!("Trees\nGot:\n{actual_tree}\nExpected:\n{n_tree}");
            assert_eq!(actual, n);
        };
    }
}

proptest! {
    #[test]
    fn display_roundtrip(mut original in node_strategy()) {
        simplify_node(&mut original);
        let string = format!("{original}");
        let actual = Expr::new(&string).unwrap();

        if original != actual {
            let actual_tree = AsTree::new(&actual);
            let n_tree = AsTree::new(&original);

            eprintln!("Got: {actual}\nExpected:{original}");
            eprintln!();
            eprintln!("Trees\nGot:\n{actual_tree}\nExpected:\n{n_tree}");
            assert_eq!(actual, original);
        };
    }
}

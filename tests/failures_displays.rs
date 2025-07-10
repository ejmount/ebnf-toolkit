use ebnf_toolkit::EbnfError;
use ebnf_toolkit::Rule;
use insta::assert_compact_debug_snapshot;

#[test]
fn incomplete_rule() {
    let src = "Foo = A|";
    let err = Rule::new(src).unwrap_err();

    println!("{err}");
    insta::assert_snapshot!(err);
}

#[test]
fn invalid_syntax_rule() {
    let srcs = ["Foo = A|;", "Foo = (A;", "Foo = (?;"];
    for src in srcs {
        let err = Rule::new(src).unwrap_err();

        println!("{err}");
        insta::assert_snapshot!(err);
    }
}

#[test]
fn invalid_start() {
    let srcs = [
        "'Hello' = A;",
        "A? = A;",
        "A* = A;",
        "? = A;",
        "#'aaa' = A;",
        "A|B = A;",
    ];
    for src in srcs {
        let err = Rule::new(src).unwrap_err();

        assert_eq!(err, err); // Custom implementation of PartialEq needs tested
        println!("{err}");
        insta::assert_snapshot!(err);
    }
}

#[test]
fn empty_input() {
    let src = "";
    let err = Rule::new(src).unwrap_err();

    println!("{err}");
    assert_eq!(err, EbnfError::EmptyInput);
}

#[test]
fn unclosed_string() {
    let input = "'Hello";

    let err = Rule::new(input).unwrap_err();

    println!("{err}");
    assert_eq!(err, err);
    assert_compact_debug_snapshot!(err);
}

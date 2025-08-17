use ebnf_toolkit::Grammar;
use insta::assert_compact_debug_snapshot;

static SRC: &str = r#"
message       ::= ['@' tags SPACE] [':' source SPACE ] command [parameters] crlf;
tags          ::= tag [';' tag]*;
tag           ::= key ['=' escaped_value];
key           ::= [ client_prefix ] [ vendor '/' ] key_name;
vendor        ::= #'[a-zA-Z0-9]+';
key_name      ::= #'[a-zA-Z0-9]+'; // this is a comment
command       ::= #'[a-zA-Z0-9]+';
escaped_value ::= #'[a-zA-Z0-9]+';
client_prefix ::= '+';
source          ::=  servername | username;
username        ::=  ( nick [ '!' user ] [ '@' host ] );
nick            ::=  #'[^ \\0\r\n #][^\\0\r\n ]*';
user            ::=  #'[^\r\n ]';
servername      ::=  #'[a-zA-Z0-9]+';
host            ::=  #'[a-zA-Z0-9.]+';
parameters      ::=  ( SPACE middle )* [ SPACE ':' trailing ];
middle          ::=  nospcrlfcl ( ':' | nospcrlfcl )*;
trailing        ::=  ( ':' | ' ' | nospcrlfcl )*;
nospcrlfcl      ::=  #'[^ :\r\n]';
SPACE           ::= ' '+;
crlf            ::= '\r\n';
"#;

#[test]
fn irc_grammar() {
    let g = Grammar::new(SRC).unwrap_or_else(|e| panic!("{e}"));

    assert_eq!(g.first_dangling_reference(), None);

    assert_compact_debug_snapshot!(g.get("tags").unwrap());
    assert_compact_debug_snapshot!(g.get("tag").unwrap());
    assert_compact_debug_snapshot!(g.get("key").unwrap());
    assert_compact_debug_snapshot!(g.get("vendor").unwrap());
    assert_compact_debug_snapshot!(g.get("key_name").unwrap());
    assert_compact_debug_snapshot!(g.get("command").unwrap());
    assert_compact_debug_snapshot!(g.get("escaped_value").unwrap());
    assert_compact_debug_snapshot!(g.get("client_prefix").unwrap());
    assert_compact_debug_snapshot!(g.get("source").unwrap());
    assert_compact_debug_snapshot!(g.get("username").unwrap());
    assert_compact_debug_snapshot!(g.get("nick").unwrap());
    assert_compact_debug_snapshot!(g.get("user").unwrap());
    assert_compact_debug_snapshot!(g.get("servername").unwrap());
    assert_compact_debug_snapshot!(g.get("host").unwrap());
    assert_compact_debug_snapshot!(g.get("parameters").unwrap());
    assert_compact_debug_snapshot!(g.get("middle").unwrap());
    assert_compact_debug_snapshot!(g.get("trailing").unwrap());
    assert_compact_debug_snapshot!(g.get("nospcrlfcl").unwrap());
    assert_compact_debug_snapshot!(g.get("SPACE").unwrap());
    assert_compact_debug_snapshot!(g.get("crlf").unwrap());
}

use ebnf_toolkit::Grammar;
use insta::assert_compact_debug_snapshot;

static SRC: &str = r#"
message       ::= ['@' tags SPACE] [':' source SPACE ] command [parameters] crlf;
tags          ::= tag [';' tag]*;
tag           ::= key ['=' escaped_value];
key           ::= [ client_prefix ] [ vendor '/' ] key_name;
vendor        ::= #'[a-zA-Z0-9]+';
key_name      ::= #'[a-zA-Z0-9]+';
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

    assert_eq!(g.get_dangling_reference(), None);

    let rules = g.rules;

    assert_compact_debug_snapshot!(rules.get("tags").unwrap());
    assert_compact_debug_snapshot!(rules.get("tag").unwrap());
    assert_compact_debug_snapshot!(rules.get("key").unwrap());
    assert_compact_debug_snapshot!(rules.get("vendor").unwrap());
    assert_compact_debug_snapshot!(rules.get("key_name").unwrap());
    assert_compact_debug_snapshot!(rules.get("command").unwrap());
    assert_compact_debug_snapshot!(rules.get("escaped_value").unwrap());
    assert_compact_debug_snapshot!(rules.get("client_prefix").unwrap());
    assert_compact_debug_snapshot!(rules.get("source").unwrap());
    assert_compact_debug_snapshot!(rules.get("username").unwrap());
    assert_compact_debug_snapshot!(rules.get("nick").unwrap());
    assert_compact_debug_snapshot!(rules.get("user").unwrap());
    assert_compact_debug_snapshot!(rules.get("servername").unwrap());
    assert_compact_debug_snapshot!(rules.get("host").unwrap());
    assert_compact_debug_snapshot!(rules.get("parameters").unwrap());
    assert_compact_debug_snapshot!(rules.get("middle").unwrap());
    assert_compact_debug_snapshot!(rules.get("trailing").unwrap());
    assert_compact_debug_snapshot!(rules.get("nospcrlfcl").unwrap());
    assert_compact_debug_snapshot!(rules.get("SPACE").unwrap());
    assert_compact_debug_snapshot!(rules.get("crlf").unwrap());
}

use ebnf_toolkit::Grammar;

static SRC: &str = r#"
message       ::= ['@' tags SPACE] [':' source SPACE ] command [parameters] crlf;
tags          ::= tag [';' tag]*;
tag           ::= key ['=' escaped_value];
key           ::= [ client_prefix ] [ vendor '/' ] key_name;
vendor        ::= #'[a-zA-Z0-9]+';
key_name      ::= #'[a-zA-Z0-9]+';
command       ::= #'[a-zA-Z0-9]+';
escaped_value ::= #'[a-zA-Z0-9]+';
client_prefix ::= '+';""
source          ::=  servername | username;
username        ::=  ( nick [ '!' user ] [ '@' host ] );
nick            ::=  #'[^ \\0\r\n #][^\\0\r\n ]*';
user            ::=  #'[^\r\n ]';
servername      ::=  #'[a-zA-Z0-9]+';
host            ::=  #'[a-zA-Z0-9.]+';
parameters      ::=  ( SPACE middle )* [ SPACE ':' trailing ];
middle          ::=  nospcrlfcl ( ':' | nospcrlfcl )*;
trailing        ::=  ( ':' | ' ' | nospcrlfcl )*;
_nospcrlfcl      ::=  #'[^ :\r\n]';
SPACE           ::= ' '+;
crlf            ::= '\r\n';
"#;

#[test]
fn irc_grammar() {
    let g = Grammar::new(SRC).unwrap_or_else(|e| panic!("{e}"));
    let mut vec: Vec<_> = g.rules.into_iter().collect();
    vec.sort_by_key(|(k, _)| *k);
    insta::assert_compact_debug_snapshot!(vec, @r"");
}

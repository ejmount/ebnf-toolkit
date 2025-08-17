# ebnf-toolkit

`ebnf-toolkit` is a Rust library for handling context-free grammars expressed in [Extended Backus–Naur form](https://en.wikipedia.org/wiki/Extended_Backus%E2%80%93Naur_form) in entirely safe code. Specifically, it can:

* parse well-formed input into a `Grammar`, the collection of syntax trees representing the EBNF rules.
* provide detailed error messages about where problems occured if input is ill-formed, with some heuristics for what may have gone wrong
* determine whether a grammar is self-contained or refers to nonterminals that have not been defined

It currently *cannot* parse input data against a given `Grammar` object, as doing this for general CFGs is very involved.

## Syntax

This library, including its syntax choices, was heavily inspired by [Kyle Lin's similar crate](https://github.com/ChAoSUnItY/ebnf) and so builds from that crate's syntax, which in turn is ultimately defined by [instaparse](https://github.com/Engelberg/instaparse). While the full details of this crate's implementation are described in the [crate documentation](https://docs.rs/ebnf-toolkit), a short illustration follows.

An EBNF *grammar* is a set of *rules*, each of which define the name of a non-terminal and the corresponding sequence of (potentially recursive) body terms which can be substituted for that name. Concretely, an `ebnf-toolkit` rule is a string looking roughly like:

```ebnf
rule_name ::= ('literal string' | other_choice) {repeating_nonterminal ','} (optional_nonterm? | #'regex');
```

Put another way, `ebnf-toolkit` deals with four types of atoms (in bold) and a number of operators that combine them:

* literal (terminal) **strings** e.g. `"hello world"` - any Unicode enclosed in single or double quotes, which can be escaped within the string by a preceding `\`. (No other escape sequences are currently processed.)
* **nonterminals** e.g. `rule_name` - a bare sequence of letters, numbers and underscores
* **regular expressions**, e.g. `#'[0-9]+'` - the part within the quotes must be a valid regular expresison as defined by the [regex](https://docs.rs/regex/latest/regex/) crate
* **terminators** - all rules end in a semicolon, `;`
* Optionals - `x?` or `[x]`, term `x` zero or one times, but not more
* Kleene stars - `x*`, the term `x` repeated any number of times, including zero
* Repetitions - `x+`, as with a Kleene star but `x` must apper at least once
* Choices - `x|y`, *either* the term `x` or the term `y`
* Group - `(xy)`, the term `x` followed directly by `y`
  * Concatenation always means the sequence of terms (`x,y` is allowed but the `,` is ignored) but this controls precedence in the usual way. That is, `xy?` *requires* `x` while `y` is optional, `(xy)?` is allowed to be empty

All whitespace is ignored. Comments are denoted by a `//` and continue to the end of the line.

## Errors

In the event that the input is malformed, error messages use [ariadne](https://crates.io/crates/ariadne) to make the cause of the problem as clear as possible.

For instance, attempting to parse `rule = (?;` as a  results and invoking `Display` on the result produces the following:

```plain
Error:
   ╭─[ <input>:1:1 ]
   │
 1 │ rule = (?;
   │        ┬┬┬
   │        ╰──── Possible unclosed bracket
   │         ││
   │         ╰─── Could not apply to preceding term
   │          │
   │          ╰── Rule ending here did not parse successfully
   │
   │ Note: The parse stack looked like this (most recent on top):
   │       └─0: UnparsedOperator [1:9..1:10]
   │         │  └─ Terminator
   │         1: UnparsedOperator [1:8..1:9]
   │         │  └─ Optional
   │         2: UnparsedOperator [1:7..1:8]
   │         │  └─ OpenedGroup
   │         3: UnparsedOperator [1:5..1:6]
   │         │  └─ Equals
   │         4: Nonterminal [1:0..1:4]
   │            └─ Rule
───╯
```

## Licence and Contributing

Licensed under either of [Apache Licence, Version 2.0](LICENSE-APACHE) or [MIT licence](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this crate by you, as defined in the Apache-2.0 licence, shall be dual licensed as above, without any additional terms or conditions.

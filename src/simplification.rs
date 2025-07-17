use display_tree::AsTree;

use crate::{Node, Rule, Span, token_data::DUMMY_SPAN};

pub(crate) fn tidy_up_rule(rule: Rule) -> Rule {
    let mut node = Node::Rule {
        span: DUMMY_SPAN,
        rule,
    };
    simplify_node(&mut node);

    let Node::Rule { rule, .. } = node else {
        unreachable!()
    };
    rule
}

pub(crate) fn simplify_node(n: &mut Node) {
    //eprintln!("{n:?}");
    n.apply_replacement(&mut flatten_groups);
    //eprintln!("{n:?}");
    n.apply_replacement(&mut flatten_choices);
    // eprintln!("{n:?}");
    // eprintln!();
}

fn flatten_groups<'a>(n: &Node<'a>) -> Option<Node<'a>> {
    match n {
        Node::Group { body, .. } if body.len() == 1 => Some(body.first().unwrap().clone()),
        Node::Optional { body, .. } if body.len() == 1 => {
            if let [Node::Group { body, .. }] = &body[..] {
                //let n = body.first().unwrap().clone();
                let span = body.iter().map(Node::span).reduce(Span::union).unwrap();

                Some(Node::Optional {
                    span,
                    body: body.clone(),
                })
            } else {
                None
            }
        }
        Node::Repeated {
            body, one_needed, ..
        } if body.len() == 1 => {
            if let [Node::Group { body, .. }] = &body[..] {
                //let n = body.first().unwrap().clone();
                let span = body.iter().map(Node::span).reduce(Span::union).unwrap();

                Some(Node::Repeated {
                    span,
                    body: body.clone(),
                    one_needed: *one_needed,
                })
            } else {
                None
            }
        }

        _ => None,
    }
}

fn flatten_choices<'a>(n: &Node<'a>) -> Option<Node<'a>> {
    if let Node::Choice { body, .. } = n {
        if body.iter().any(|m| matches!(m, Node::Choice { .. })) {
            let mut outputs = vec![];
            for child in body {
                match child {
                    Node::Choice { body, .. } => outputs.extend(body.iter().cloned()),
                    other => outputs.push(other.clone()),
                }
            }
            let span = outputs.iter().map(Node::span).reduce(Span::union).unwrap();
            Some(Node::Choice {
                span,
                body: outputs,
            })
        } else {
            None
        }
    } else {
        None
    }
}

#[test]
fn flatten_choice1() {
    use Node::*;
    let val = Choice {
        span: DUMMY_SPAN,
        body: vec![
            Choice {
                span: DUMMY_SPAN,
                body: vec![
                    Nonterminal {
                        span: DUMMY_SPAN,
                        name: "nonterminal0027",
                    },
                    Nonterminal {
                        span: DUMMY_SPAN,
                        name: "nonterminal0028",
                    },
                ],
            },
            Terminal {
                span: DUMMY_SPAN,
                str: "literal1",
            },
        ],
    };

    let mut simplified = val.clone();
    simplify_node(&mut simplified);

    println!("{}", AsTree::new(&simplified));
}

#[allow(warnings)]
#[test]
fn foo() {
    use Node::*;
    let original_val = Group {
        span: DUMMY_SPAN,
        body: vec![
            Group {
                span: DUMMY_SPAN,
                body: vec![Choice {
                    span: DUMMY_SPAN,
                    body: vec![
                        Choice {
                            span: DUMMY_SPAN,
                            body: vec![
                                Choice {
                                    span: DUMMY_SPAN,
                                    body: vec![
                                        Choice {
                                            span: DUMMY_SPAN,
                                            body: vec![
                                                Choice {
                                                    span: DUMMY_SPAN,
                                                    body: vec![
                                                        Choice {
                                                            span: DUMMY_SPAN,
                                                            body: vec![Choice { span: DUMMY_SPAN
, body: vec![Choice { span: DUMMY_SPAN
, body: vec![Choice { span: DUMMY_SPAN
, body: vec![Nonterminal { span: DUMMY_SPAN
, name: "nonterminal0027" }
, Nonterminal
{ span: DUMMY_SPAN
, name: "nonterminal0028" }] }
, Terminal { span: DUMMY_SPAN
, str: "literal" }] }
, Nonterminal { span: DUMMY_SPAN
, name: "nonterminal0029" }] }
, Terminal { span: DUMMY_SPAN
, str: "literal" }],
                                                        },
                                                        Terminal {
                                                            span: DUMMY_SPAN,
                                                            str: "literal",
                                                        },
                                                    ],
                                                },
                                                Nonterminal {
                                                    span: DUMMY_SPAN,
                                                    name: "regex",
                                                },
                                            ],
                                        },
                                        Terminal {
                                            span: DUMMY_SPAN,
                                            str: "literal",
                                        },
                                    ],
                                },
                                Nonterminal {
                                    span: DUMMY_SPAN,
                                    name: "nonterminal0030",
                                },
                            ],
                        },
                        Nonterminal {
                            span: DUMMY_SPAN,
                            name: "regex",
                        },
                    ],
                }],
            },
            Group {
                span: DUMMY_SPAN,
                body: vec![
                    Nonterminal {
                        span: DUMMY_SPAN,
                        name: "nonterminal0031",
                    },
                    Nonterminal {
                        span: DUMMY_SPAN,
                        name: "nonterminal0032",
                    },
                    Nonterminal {
                        span: DUMMY_SPAN,
                        name: "regex",
                    },
                    Nonterminal {
                        span: DUMMY_SPAN,
                        name: "nonterminal0033",
                    },
                    Nonterminal {
                        span: DUMMY_SPAN,
                        name: "nonterminal0034",
                    },
                    Nonterminal {
                        span: DUMMY_SPAN,
                        name: "nonterminal0035",
                    },
                    Nonterminal {
                        span: DUMMY_SPAN,
                        name: "nonterminal0036",
                    },
                    Nonterminal {
                        span: DUMMY_SPAN,
                        name: "regex",
                    },
                    Nonterminal {
                        span: DUMMY_SPAN,
                        name: "regex",
                    },
                    Nonterminal {
                        span: DUMMY_SPAN,
                        name: "regex",
                    },
                ],
            },
            Group {
                span: DUMMY_SPAN,
                body: vec![
                    Terminal {
                        span: DUMMY_SPAN,
                        str: "literal",
                    },
                    Nonterminal {
                        span: DUMMY_SPAN,
                        name: "regex",
                    },
                    Nonterminal {
                        span: DUMMY_SPAN,
                        name: "regex",
                    },
                    Nonterminal {
                        span: DUMMY_SPAN,
                        name: "nonterminal0037",
                    },
                    Terminal {
                        span: DUMMY_SPAN,
                        str: "literal",
                    },
                    Nonterminal {
                        span: DUMMY_SPAN,
                        name: "regex",
                    },
                    Nonterminal {
                        span: DUMMY_SPAN,
                        name: "nonterminal0038",
                    },
                    Nonterminal {
                        span: DUMMY_SPAN,
                        name: "regex",
                    },
                    Nonterminal {
                        span: DUMMY_SPAN,
                        name: "regex",
                    },
                    Nonterminal {
                        span: DUMMY_SPAN,
                        name: "regex",
                    },
                ],
            },
            Nonterminal {
                span: DUMMY_SPAN,
                name: "regex",
            },
            Nonterminal {
                span: DUMMY_SPAN,
                name: "nonterminal0039",
            },
            Nonterminal {
                span: DUMMY_SPAN,
                name: "regex",
            },
            Nonterminal {
                span: DUMMY_SPAN,
                name: "regex",
            },
            Nonterminal {
                span: DUMMY_SPAN,
                name: "regex",
            },
            Group {
                span: DUMMY_SPAN,
                body: vec![Choice {
                    span: DUMMY_SPAN,
                    body: vec![
                        Choice {
                            span: DUMMY_SPAN,
                            body: vec![
                                Choice {
                                    span: DUMMY_SPAN,
                                    body: vec![
                                        Choice {
                                            span: DUMMY_SPAN,
                                            body: vec![
                                                Choice {
                                                    span: DUMMY_SPAN,
                                                    body: vec![
                                                        Choice {
                                                            span: DUMMY_SPAN,
                                                            body: vec![Choice { span: DUMMY_SPAN
, body: vec![Choice { span: DUMMY_SPAN
, body: vec![Choice { span: DUMMY_SPAN
, body: vec![Terminal { span: DUMMY_SPAN
, str: "literal" }
, Nonterminal { span: DUMMY_SPAN
, name: "nonterminal0040" }] }
, Nonterminal { span: DUMMY_SPAN
, name: "regex" }] }
, Nonterminal { span: DUMMY_SPAN
, name: "nonterminal0041" }] }
, Nonterminal { span: DUMMY_SPAN
, name: "regex" }],
                                                        },
                                                        Terminal {
                                                            span: DUMMY_SPAN,
                                                            str: "literal",
                                                        },
                                                    ],
                                                },
                                                Terminal {
                                                    span: DUMMY_SPAN,
                                                    str: "literal",
                                                },
                                            ],
                                        },
                                        Terminal {
                                            span: DUMMY_SPAN,
                                            str: "literal",
                                        },
                                    ],
                                },
                                Terminal {
                                    span: DUMMY_SPAN,
                                    str: "literal",
                                },
                            ],
                        },
                        Nonterminal {
                            span: DUMMY_SPAN,
                            name: "nonterminal0042",
                        },
                    ],
                }],
            },
            Optional {
                span: DUMMY_SPAN,
                body: vec![Group {
                    span: DUMMY_SPAN,
                    body: vec![
                        Nonterminal {
                            span: DUMMY_SPAN,
                            name: "nonterminal0043",
                        },
                        Nonterminal {
                            span: DUMMY_SPAN,
                            name: "regex",
                        },
                        Nonterminal {
                            span: DUMMY_SPAN,
                            name: "regex",
                        },
                        Nonterminal {
                            span: DUMMY_SPAN,
                            name: "nonterminal0044",
                        },
                        Nonterminal {
                            span: DUMMY_SPAN,
                            name: "nonterminal0045",
                        },
                        Terminal {
                            span: DUMMY_SPAN,
                            str: "literal",
                        },
                        Nonterminal {
                            span: DUMMY_SPAN,
                            name: "nonterminal0046",
                        },
                        Nonterminal {
                            span: DUMMY_SPAN,
                            name: "nonterminal0047",
                        },
                        Nonterminal {
                            span: DUMMY_SPAN,
                            name: "regex",
                        },
                        Nonterminal {
                            span: DUMMY_SPAN,
                            name: "regex",
                        },
                    ],
                }],
            },
        ],
    };

    let mut simplified = original_val.clone();
    simplify_node(&mut simplified);

    let original_tree = AsTree::new(&original_val);
    let simple_tree = AsTree::new(&simplified);

    let original_tree = format!("{original_tree}");
    let simple_tree = format!("{simple_tree}");

    let original_tree: Vec<_> = original_tree.lines().collect();
    let simple_tree: Vec<_> = simple_tree.lines().collect();

    let max_width = original_tree
        .iter()
        .map(|s| s.len())
        .reduce(usize::max)
        .unwrap();

    for n in 0..original_tree.len().max(simple_tree.len()) {
        println!(
            "{:max_width$}    {}",
            original_tree.get(n).map_or("", |v| v),
            simple_tree.get(n).map_or("", |v| v)
        )
    }

    //assert_ne!(simplified, original_val);
    //    todo!()
}

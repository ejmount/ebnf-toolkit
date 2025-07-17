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
    n.apply_replacement(&mut flatten_groups);
    n.apply_replacement(&mut flatten_choices);
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

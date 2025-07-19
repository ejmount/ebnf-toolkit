use crate::{Node, Span};

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
                let span = Span::union(body.iter());

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
                let span = Span::union(body.iter());

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
    if let Node::Choice { body, .. } = n
        && body.iter().any(|m| matches!(m, Node::Choice { .. }))
    {
        let mut outputs = vec![];
        for child in body {
            match child {
                Node::Choice { body, .. } => outputs.extend(body.iter().cloned()),
                other => outputs.push(other.clone()),
            }
        }
        let span = Span::union(outputs.iter());
        Some(Node::Choice {
            span,
            body: outputs,
        })
    } else {
        None
    }
}

#[cfg(test)]
mod test {
    use display_tree::AsTree;

    use crate::{simplification::simplify_node, token_data::DUMMY_SPAN};
    #[test]
    fn flatten_choice1() {
        use crate::Node::*;
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

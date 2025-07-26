use crate::{Expr, Span};

pub(crate) fn simplify_node(n: &mut Expr) {
    n.apply_replacement(&mut flatten_groups);
    n.apply_replacement(&mut flatten_choices);
}

fn flatten_groups<'a>(n: &Expr<'a>) -> Option<Expr<'a>> {
    match n {
        Expr::Group { body, .. } if body.len() == 1 => Some(body.first().unwrap().clone()),
        Expr::Optional { body, .. } if body.len() == 1 => {
            if let [Expr::Group { body, .. }] = &body[..] {
                let span = Span::union(body.iter());

                Some(Expr::Optional {
                    span,
                    body: body.clone(),
                })
            } else {
                None
            }
        }
        Expr::Repetition {
            body, one_needed, ..
        } if body.len() == 1 => {
            if let [Expr::Group { body, .. }] = &body[..] {
                let span = Span::union(body.iter());

                Some(Expr::Repetition {
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

fn flatten_choices<'a>(n: &Expr<'a>) -> Option<Expr<'a>> {
    if let Expr::Choice { body, .. } = n
        && body.iter().any(|m| matches!(m, Expr::Choice { .. }))
    {
        let mut outputs = vec![];
        for child in body {
            match child {
                Expr::Choice { body, .. } => outputs.extend(body.iter().cloned()),
                other => outputs.push(other.clone()),
            }
        }
        let span = Span::union(outputs.iter());
        Some(Expr::Choice {
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
        use crate::Expr::*;
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
                Literal {
                    span: DUMMY_SPAN,
                    str: "literal1",
                },
            ],
        };

        let mut simplified = val.clone();
        simplify_node(&mut simplified);

        println!("{}", AsTree::new(&simplified));
    }
}

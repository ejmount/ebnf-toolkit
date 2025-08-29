use crate::{Expr, Span, expr::ExprKind};

pub(crate) fn simplify_node(n: &mut Expr) {
    n.apply_replacement(&mut remove_redundant_layers);
    n.apply_replacement(&mut flatten_choices);
}

fn remove_redundant_layers<'a>(n: &Expr<'a>) -> Option<Expr<'a>> {
    fn flatten_groups<'a>(groups: &[Expr<'a>]) -> Vec<Expr<'a>> {
        let mut new_body = vec![];
        for expr in groups {
            match &expr {
                Expr::Group { body, .. } => new_body.extend(body.iter().cloned()),
                &other => new_body.push(other.clone()),
            }
        }
        new_body
    }
    match n {
        Expr::Group { body, .. } => {
            let new_body = flatten_groups(body);
            if new_body.len() == 1 {
                Some(new_body.first().unwrap().clone())
            } else {
                Some(Expr::Group {
                    span: Span::union(new_body.iter()),
                    body: new_body,
                })
            }
        }
        Expr::Optional { body, .. } => {
            if body.iter().any(|e| ExprKind::from(e) == ExprKind::Group) {
                let new_body = flatten_groups(body);
                let span = Span::union(new_body.iter());

                Some(Expr::Optional {
                    span,
                    body: new_body,
                })
            } else {
                None
            }
        }
        Expr::Repetition {
            body, one_needed, ..
        } => {
            if body.iter().any(|e| ExprKind::from(e) == ExprKind::Group) {
                let new_body = flatten_groups(body);
                let span = Span::union(new_body.iter());

                Some(Expr::Repetition {
                    span,
                    body: new_body,
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

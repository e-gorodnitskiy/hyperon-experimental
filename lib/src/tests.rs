#![allow(non_snake_case)]

use super::*;

// Aliases to have a shorter notation
fn S(name: &str) -> Atom { Atom::sym(name) }
fn E(children: &[Atom]) -> Atom { Atom::expr(children) }
fn V(name: &str) -> Atom { Atom::var(name) }

#[test]
fn test_expr_symbol() {
    assert_eq!(expr!("="), S("="));
    assert_eq!(expr!("1"), S("1"));
    assert_eq!(expr!("*"), S("*"));
    assert_eq!(expr!("foo"), S("foo"));
}

#[test]
fn test_expr_variable() {
    assert_eq!(expr!(n), V("n"));
    assert_eq!(expr!(self), V("self"));
}

#[test]
fn test_expr_expression() {
    assert_eq!(expr!("=", ("fact", n), ("*", n, ("-", n, "1"))), 
               E(&[S("="), E(&[S("fact"), V("n")]),
               E(&[ S("*"), V("n"), E(&[ S("-"), V("n"), S("1") ]) ]) ]));
}

#[test]
fn test_grounded() {
    assert_eq!(Atom::gnd(3), Atom::Grounded(Box::new(3)));
}

#[test]
fn test_display_symbol() {
    assert_eq!(format!("{}", Atom::sym("test")), "test");
}

#[test]
fn test_display_variable() {
    assert_eq!(format!("{}", Atom::var("x")), "$x");
}

#[test]
fn test_display_expression() {
    assert_eq!(format!("{}", expr!("=", ("fact", n), ("*", n, ("-", n, "1")))),
        "(= (fact $n) (* $n (- $n 1)))");
    assert_eq!(format!("{}", expr!()), "()");
}

#[test]
fn test_display_grounded() {
    assert_eq!(format!("{}", Atom::gnd(42)), "42");
}

#[test]
fn test_match_symbol() {
    let mut space = GroundingSpace::new();
    space.add(expr!("foo"));
    assert_eq!(space.query(&expr!("foo")), vec![bind!{}]);
}

#[test]
fn test_match_variable() {
    let mut space = GroundingSpace::new();
    space.add(expr!("foo"));
    assert_eq!(space.query(&expr!(x)), vec![bind!{x: expr!("foo")}]);
}

#[test]
fn test_match_expression() {
    let mut space = GroundingSpace::new();
    space.add(expr!("+", "a", ("*", "b", "c")));
    assert_eq!(space.query(&expr!("+", "a", ("*", "b", "c"))), vec![bind!{}]);
}

#[test]
fn test_match_expression_with_variables() {
    let mut space = GroundingSpace::new();
    space.add(expr!("+", "A", ("*", "B", "C")));
    assert_eq!(space.query(&expr!("+", a, ("*", b, c))),
        vec![bind!{a: expr!("A"), b: expr!("B"), c: expr!("C") }]);
}

#[test]
fn test_match_different_value_for_variable() {
    let mut space = GroundingSpace::new();
    space.add(expr!("+", "A", ("*", "B", "C")));
    assert_eq!(space.query(&expr!("+", a, ("*", a, c))), vec![]);
}

#[test]
fn test_match_query_variable_has_priority() {
    let mut space = GroundingSpace::new();
    space.add(expr!("equals", x, x));
    assert_eq!(space.query(&expr!("equals", y, z)), vec![bind!{y: expr!(x), z: expr!(x)}]);
}

#[test]
fn test_match_query_variable_via_data_variable() {
    let mut space = GroundingSpace::new();
    space.add(expr!(x, x));
    assert_eq!(space.query(&expr!(y, (z))), vec![bind!{y: expr!((z))}]);
}

#[test]
fn test_subexpression_iterator() {
    let expr = expr!("+", ("*", "3", ("+", "1", n)), ("-", "4", "3"));

    let iter = expr.as_expr().unwrap().sub_expr_iter();

    assert_eq!(iter.collect::<Vec<_>>(),
        vec![
            expr!("+", "1", n),
            expr!("*", "3", ("+", "1", n)),
            expr!("-", "4", "3"),
            expr!("+", ("*", "3", ("+", "1", n)), ("-", "4", "3")),
        ]);
}

#[test]
fn test_subexpression_iterator_two_sub_expr() {
    let expr = expr!("*", ("+", "3", "4"), ("-", "5", "2"));

    let iter = expr.as_expr().unwrap().sub_expr_iter();

    assert_eq!(iter.collect::<Vec<_>>(),
        vec![
            expr!("+", "3", "4"),
            expr!("-", "5", "2"),
            expr!("*", ("+", "3", "4"), ("-", "5", "2")),
        ]);
}
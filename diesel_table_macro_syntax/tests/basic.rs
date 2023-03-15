use diesel_table_macro_syntax::*;

#[test]
fn basic() {
    let input = include_str!("basic.rs.in");
    let t: TableDecl = syn::parse_str(input).unwrap();
}

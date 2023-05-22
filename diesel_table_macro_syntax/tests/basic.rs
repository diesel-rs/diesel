use diesel_table_macro_syntax::*;

#[test]
fn basic() {
    let input = include_str!("basic.rs.in");
    let t: TableDecl = syn::parse_str(input).unwrap();
    assert_eq!(t.column_defs.len(), 3);
    assert_eq!(
        t.column_defs
            .iter()
            .map(|c| c
                .max_length
                .as_ref()
                .map(|n| n.base10_parse::<usize>().unwrap()))
            .collect::<Vec<_>>(),
        &[None, Some(120), Some(120)]
    )
}

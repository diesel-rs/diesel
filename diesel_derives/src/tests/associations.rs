use super::derive;

use super::expand_with;

#[test]
pub(crate) fn associations_1() {
    let input = quote::quote! {
        #[diesel(belongs_to(User))]
        struct Post {
            id: i32,
            title: String,
            user_id: i32,
        }
    };

    expand_with(
        &crate::derive_associations_inner as &dyn Fn(_) -> _,
        input,
        derive(syn::parse_quote!(#[derive(Associations)])),
        "associations_1",
    );
}

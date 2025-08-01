trait ProcMacroFn {
    type Input: Clone;

    fn call(&self, input: Self::Input) -> proc_macro2::TokenStream;
}

impl ProcMacroFn for &dyn Fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    type Input = proc_macro2::TokenStream;

    fn call(&self, input: Self::Input) -> proc_macro2::TokenStream {
        (self)(input)
    }
}

impl ProcMacroFn
    for &dyn Fn(proc_macro2::TokenStream, proc_macro2::TokenStream) -> proc_macro2::TokenStream
{
    type Input = (proc_macro2::TokenStream, proc_macro2::TokenStream);

    fn call(&self, (attrs, input): Self::Input) -> proc_macro2::TokenStream {
        (self)(attrs, input)
    }
}

trait AttributeFormatter<I> {
    fn format(&self, input: I) -> String;
}

impl AttributeFormatter<proc_macro2::TokenStream> for syn::Attribute {
    fn format(&self, input: proc_macro2::TokenStream) -> String {
        format_input(quote::quote! {
            #self
            #input
        })
    }
}

fn derive(attr: syn::Attribute) -> syn::Attribute {
    attr
}

struct FunctionMacro(syn::Ident);

impl AttributeFormatter<proc_macro2::TokenStream> for FunctionMacro {
    fn format(&self, input: proc_macro2::TokenStream) -> String {
        let name = &self.0;
        format_input(quote::quote! {
            #name! {
                #input
            }
        })
    }
}

struct AttributeMacro(syn::Path);

impl AttributeFormatter<(proc_macro2::TokenStream, proc_macro2::TokenStream)> for AttributeMacro {
    fn format(
        &self,
        (attr, input): (proc_macro2::TokenStream, proc_macro2::TokenStream),
    ) -> String {
        let attr_ident = &self.0;
        let tokens = if attr.is_empty() {
            quote::quote! {
                #[#attr_ident]
                #input
            }
        } else {
            quote::quote! {
                #[#attr_ident(#attr)]
                #input
            }
        };
        format_input(tokens)
    }
}

fn format_input(input: proc_macro2::TokenStream) -> String {
    let input_string = syn::parse2(input.clone())
        .as_ref()
        .map(prettyplease::unparse)
        .unwrap_or_else(|_| input.to_string());
    input_string
}

#[track_caller]
fn expand_with<Fn: ProcMacroFn>(
    function: Fn,
    input: Fn::Input,
    attribute: impl AttributeFormatter<Fn::Input>,
    snapshot_name: &str,
) {
    let input_string = attribute.format(input.clone());
    let out = function.call(input);

    let file = syn::parse2(out).unwrap();

    let out = prettyplease::unparse(&file);
    let mut settings = insta::Settings::new();
    let content = vec![(
        insta::internals::Content::String("input".into()),
        insta::internals::Content::String(input_string),
    )];

    settings.set_raw_info(&insta::internals::Content::Map(content));

    let _scope = settings.bind_to_scope();

    insta::assert_snapshot!(snapshot_name, out);
}

mod as_changeset;
mod as_expression;
mod associations;
mod auto_type;
mod declare_sql_function;
mod define_sql_function;
mod diesel_for_each_tuple;
mod diesel_numeric_ops;
mod diesel_public_if;
mod from_sql_row;
mod has_query;
mod identifiable;
mod insertable;
mod multiconnection;
mod query_id;
mod queryable;
mod queryable_by_name;
mod selectable;
mod sql_function;
mod sql_type;
mod table;
mod valid_grouping;

use syn::visit_mut::VisitMut;

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
    syn::parse2(input.clone())
        .as_ref()
        .map(prettyplease::unparse)
        .unwrap_or_else(|_| input.to_string())
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

    let mut file = syn::parse2(out).unwrap();
    let mut visitor = FixupVisitor;
    visitor.visit_file_mut(&mut file);

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

struct FixupVisitor;

impl syn::visit_mut::VisitMut for FixupVisitor {
    fn visit_stmt_mut(&mut self, i: &mut syn::Stmt) {
        if let syn::Stmt::Macro(m) = i {
            let macro_ident = m.mac.path.segments.last().map(|c| c.ident.to_string());
            match macro_ident.as_deref() {
                Some("expand_sqlite") if cfg!(feature = "sqlite") => {
                    let item = syn::parse2(m.mac.tokens.clone()).unwrap();
                    *i = syn::Stmt::Item(item);
                }
                Some("expand_mysql") if cfg!(feature = "mysql") => {
                    let item = syn::parse2(m.mac.tokens.clone()).unwrap();
                    *i = syn::Stmt::Item(item);
                }
                Some("expand_pg") if cfg!(feature = "postgres") => {
                    let item = syn::parse2(m.mac.tokens.clone()).unwrap();
                    *i = syn::Stmt::Item(item);
                }
                Some("expand_r2d2") if cfg!(feature = "r2d2") => {
                    let item = syn::parse2(m.mac.tokens.clone()).unwrap();
                    *i = syn::Stmt::Item(item);
                }
                Some("expand_chrono") if cfg!(feature = "chrono") => {
                    let item = syn::parse2(m.mac.tokens.clone()).unwrap();
                    *i = syn::Stmt::Item(item);
                }
                Some("expand_time") if cfg!(feature = "time") => {
                    let item = syn::parse2(m.mac.tokens.clone()).unwrap();
                    *i = syn::Stmt::Item(item);
                }
                Some("expand_numeric") if cfg!(feature = "numeric") => {
                    let item = syn::parse2(m.mac.tokens.clone()).unwrap();
                    *i = syn::Stmt::Item(item);
                }

                Some(
                    "expand_sqlite" | "expand_pg" | "expand_mysql" | "expand_r2d2"
                    | "expand_chrono" | "expand_time" | "expand_numeric",
                ) => {
                    *i = syn::Stmt::Item(syn::Item::Verbatim(proc_macro2::TokenStream::new()));
                }
                Some(_) | None => {}
            }
        }
        syn::visit_mut::visit_stmt_mut(self, i);
    }

    fn visit_item_mut(&mut self, i: &mut syn::Item) {
        if let syn::Item::Macro(m) = i {
            let macro_ident = m.mac.path.segments.last().map(|c| c.ident.to_string());
            match macro_ident.as_deref() {
                Some("expand_sqlite") if cfg!(feature = "sqlite") => {
                    *i = syn::parse2(m.mac.tokens.clone()).unwrap();
                }
                Some("expand_mysql") if cfg!(feature = "mysql") => {
                    *i = syn::parse2(m.mac.tokens.clone()).unwrap();
                }
                Some("expand_pg") if cfg!(feature = "postgres") => {
                    *i = syn::parse2(m.mac.tokens.clone()).unwrap();
                }
                Some("expand_r2d2") if cfg!(feature = "r2d2") => {
                    *i = syn::parse2(m.mac.tokens.clone()).unwrap();
                }
                Some("expand_chrono") if cfg!(feature = "chrono") => {
                    *i = syn::parse2(m.mac.tokens.clone()).unwrap();
                }
                Some("expand_time") if cfg!(feature = "time") => {
                    *i = syn::parse2(m.mac.tokens.clone()).unwrap();
                }
                Some("expand_sqlite_function") if cfg!(feature = "sqlite") => {
                    let tokens = m.mac.tokens.clone().into_iter().skip(2).collect();
                    *i = syn::parse2(tokens).unwrap();
                }
                Some("expand_numeric") if cfg!(feature = "numeric") => {
                    *i = syn::parse2(m.mac.tokens.clone()).unwrap();
                }
                Some(
                    "expand_sqlite"
                    | "expand_pg"
                    | "expand_mysql"
                    | "expand_r2d2"
                    | "expand_time"
                    | "expand_chrono"
                    | "expand_sqlite_function"
                    | "expand_numeric",
                ) => {
                    *i = syn::Item::Verbatim(proc_macro2::TokenStream::new());
                }
                Some(_) | None => {}
            }
        }
        syn::visit_mut::visit_item_mut(self, i);
    }
}

mod allow_tables_to_appear_in_same_query;
mod as_changeset;
mod as_expression;
mod associations;
mod auto_type;
mod declare_sql_function;
mod define_sql_function;
mod diesel_for_each_tuple;
mod diesel_numeric_ops;
mod diesel_public_if;
mod enum_;
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
mod view;

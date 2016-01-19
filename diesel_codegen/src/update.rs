use aster;
use syntax::ast::{self, MetaItem, TyPath};
use syntax::attr::AttrMetaMethods;
use syntax::codemap::Span;
use syntax::ext::base::{Annotatable, ExtCtxt};
use syntax::ptr::P;
use syntax::parse::token::{InternedString, intern_and_get_ident, str_to_ident};

use attr::Attr;
use model::Model;

pub fn expand_changeset_for(
    cx: &mut ExtCtxt,
    span: Span,
    meta_item: &MetaItem,
    annotatable: &Annotatable,
    push: &mut FnMut(Annotatable),
) {
    let builder = aster::AstBuilder::new().span(span);

    if let Some(model) = Model::from_annotable(cx, &builder, annotatable) {
        let options = changeset_options(cx, meta_item).unwrap();
        push(Annotatable::Item(changeset_impl(cx, builder, &options, &model).unwrap()));
        if let Some(item) = save_changes_impl(cx, builder, &options, &model) {
            push(Annotatable::Item(item));
        }
    } else {
        cx.span_err(meta_item.span,
            "`changeset_for` may only be apllied to enums and structs");
    }
}

struct ChangesetOptions {
    table_name: ast::Ident,
    skip_visibility: bool,
}

fn changeset_options(cx: &mut ExtCtxt, meta_item: &MetaItem) -> Result<ChangesetOptions, ()> {
    match meta_item.node {
        ast::MetaList(_, ref meta_items) => {
            let table_name = try!(table_name(cx, &meta_items[0]));
            let skip_visibility = try!(boolean_option(cx, &meta_items[1..], "__skip_visibility"))
                .unwrap_or(false);
            Ok(ChangesetOptions {
                table_name: str_to_ident(&table_name),
                skip_visibility: skip_visibility,
            })
        }
        _ => usage_error(cx, meta_item),
    }
}

fn table_name(cx: &mut ExtCtxt, meta_item: &MetaItem) -> Result<InternedString, ()> {
    match meta_item.node {
        ast::MetaWord(ref word) => Ok(word.clone()),
        _ => usage_error(cx, meta_item),
    }
}

fn boolean_option(cx: &mut ExtCtxt, meta_items: &[P<MetaItem>], option_name: &str)
    -> Result<Option<bool>, ()>
{
    if let Some(item) = meta_items.iter().find(|item| item.name() == option_name) {
        match item.value_str() {
            Some(ref s) if *s == "true" => Ok(Some(true)),
            Some(ref s) if *s == "false" => Ok(Some(false)),
            _ => {
                cx.span_err(item.span,
                    &format!("Expected {} to be in the form `option=\"true\"` or \
                            option=\"false\"", option_name));
                Err(())
            }
        }
    } else {
        Ok(None)
    }
}

fn usage_error<T>(cx: &mut ExtCtxt, meta_item: &MetaItem) -> Result<T, ()> {
    cx.span_err(meta_item.span,
        "`changeset_for` must be used in the form `#[changeset_for(table1)]`");
    Err(())
}

fn changeset_impl(
    cx: &mut ExtCtxt,
    builder: aster::AstBuilder,
    options: &ChangesetOptions,
    model: &Model,
) -> Option<P<ast::Item>> {
    let table: &str = &options.table_name.name.as_str();
    let ref struct_name = model.ty;
    let pk = model.primary_key_name();
    let attrs_for_changeset = model.attrs.iter().filter(|a| a.column_name != pk)
        .collect::<Vec<_>>();
    let changeset_ty = builder.ty().tuple()
        .with_tys(attrs_for_changeset.iter()
                  .map(|a| changeset_ty(cx, builder, table, a)))
        .build();
    let changeset_body = builder.expr().tuple()
        .with_exprs(attrs_for_changeset.iter()
                    .map(|a| changeset_expr(cx, builder, table, a)))
        .build();
    quote_item!(cx,
        impl<'a: 'update, 'update> ::diesel::query_builder::AsChangeset for
            &'update $struct_name
        {
            type Changeset = $changeset_ty;

            fn as_changeset(self) -> Self::Changeset {
                $changeset_body
            }
        }
    )
}

#[allow(unused_mut)]
fn save_changes_impl(
    cx: &mut ExtCtxt,
    builder: aster::AstBuilder,
    options: &ChangesetOptions,
    model: &Model,
) -> Option<P<ast::Item>> {
    let ref struct_name = model.ty;
    let pk = model.primary_key_name();
    let sql_type = builder.path()
        .segment(&options.table_name).build()
        .segment("SqlType").build()
        .build();
    let table = builder.path()
        .segment(&options.table_name).build()
        .segment("table").build()
        .build();
    let _pub = if options.skip_visibility {
        quote_tokens!(cx, )
    } else {
        quote_tokens!(cx, pub)
    };
    model.attrs.iter().find(|a| a.column_name == pk).and_then(|pk| {
        let pk_field = pk.field_name.unwrap();
        quote_item!(cx,
            impl<'a> $struct_name {
                $_pub fn save_changes<T>(&self, connection: &::diesel::Connection)
                    -> ::diesel::QueryResult<T> where
                    T: Queryable<$sql_type>,
                {
                    use ::diesel::update;
                    update($table.filter($table.primary_key().eq(&self.$pk_field)))
                        .set(self)
                        .get_result(&connection)
                }
            }
        )
    })
}

fn changeset_ty(
    cx: &mut ExtCtxt,
    builder: aster::AstBuilder,
    table: &str,
    attr: &Attr,
) -> P<ast::Ty> {
    let column = builder.path()
        .segment(table).build()
        .segment(attr.column_name).build()
        .build();
    if let Some(ty) = ty_param_of_option(&attr.ty) {
        let inner_ty = inner_changeset_ty(cx, column, &ty);
        quote_ty!(cx, Option<$inner_ty>)
    } else {
        inner_changeset_ty(cx, column, &attr.ty)
    }
}

fn inner_changeset_ty(
    cx: &mut ExtCtxt,
    column: ast::Path,
    field_ty: &ast::Ty,
) -> P<ast::Ty> {
    quote_ty!(cx,
        ::diesel::expression::predicates::Eq<
            $column,
            ::diesel::expression::bound::Bound<
                <$column as ::diesel::expression::Expression>::SqlType,
                &'update $field_ty,
            >,
        >
    )
}

fn changeset_expr(
    cx: &mut ExtCtxt,
    builder: aster::AstBuilder,
    table: &str,
    attr: &Attr,
) -> P<ast::Expr> {
    let column = builder.path()
        .segment(table).build()
        .segment(attr.column_name).build()
        .build();
    let field_name = &attr.field_name.unwrap();
    if is_option_ty(&attr.ty) {
        quote_expr!(cx, self.$field_name.as_ref().map(|f| $column.eq(f)))
    } else {
        quote_expr!(cx, $column.eq(&self.$field_name))
    }
}

fn ty_param_of_option(ty: &ast::Ty) -> Option<&P<ast::Ty>> {
    match ty.node {
        TyPath(_, ref path) => {
            path.segments.first().iter()
                .filter(|s| s.identifier.name.as_str() == intern_and_get_ident("Option"))
                .flat_map(|s| s.parameters.types().first().map(|p| *p))
                .next()
        }
        _ => None,
    }
}

fn is_option_ty(ty: &ast::Ty) -> bool {
    ty_param_of_option(ty).is_some()
}

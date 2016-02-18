use syntax::ast::{self, MetaItem, MetaItemKind, TyKind};
use syntax::attr::AttrMetaMethods;
use syntax::codemap::Span;
use syntax::ext::base::{Annotatable, ExtCtxt};
use syntax::ext::build::AstBuilder;
use syntax::ptr::P;
use syntax::parse::token::{InternedString, str_to_ident};

use attr::Attr;
use model::Model;
use util::{ty_param_of_option, is_option_ty};

pub fn expand_changeset_for(
    cx: &mut ExtCtxt,
    span: Span,
    meta_item: &MetaItem,
    annotatable: &Annotatable,
    push: &mut FnMut(Annotatable),
) {
    if let Some(model) = Model::from_annotable(cx, span, annotatable) {
        let options = changeset_options(cx, meta_item).unwrap();
        push(Annotatable::Item(changeset_impl(cx, span, &options, &model).unwrap()));
        let items = save_changes_impl(cx, span, &options, &model);
        for item in items.into_iter().filter_map(|x| x) {
            push(Annotatable::Item(item));
        }
    } else {
        cx.span_err(meta_item.span,
            "`changeset_for` may only be apllied to enums and structs");
    }
}

struct ChangesetOptions {
    table_name: ast::Ident,
    treat_none_as_null: bool,
}

fn changeset_options(cx: &mut ExtCtxt, meta_item: &MetaItem) -> Result<ChangesetOptions, ()> {
    match meta_item.node {
        MetaItemKind::List(_, ref meta_items) => {
            let table_name = try!(table_name(cx, &meta_items[0]));
            let treat_none_as_null = try!(boolean_option(cx, &meta_items[1..], "treat_none_as_null"))
                .unwrap_or(false);
            Ok(ChangesetOptions {
                table_name: str_to_ident(&table_name),
                treat_none_as_null: treat_none_as_null,
            })
        }
        _ => usage_error(cx, meta_item),
    }
}

fn table_name(cx: &mut ExtCtxt, meta_item: &MetaItem) -> Result<InternedString, ()> {
    match meta_item.node {
        MetaItemKind::Word(ref word) => Ok(word.clone()),
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
    span: Span,
    options: &ChangesetOptions,
    model: &Model,
) -> Option<P<ast::Item>> {
    let ref struct_name = model.ty;
    let pk = model.primary_key_name();
    let table_name = options.table_name;
    let attrs_for_changeset = model.attrs.iter().filter(|a| a.column_name != pk)
        .collect::<Vec<_>>();
    let changeset_ty = cx.ty(span, TyKind::Tup(
        attrs_for_changeset.iter()
              .map(|a| changeset_ty(cx, span, &options, a))
              .collect()
    ));
    let changeset_body = cx.expr_tuple(span, attrs_for_changeset.iter()
        .map(|a| changeset_expr(cx, span, &options, a))
        .collect());
    quote_item!(cx,
        impl<'a: 'update, 'update> ::diesel::query_builder::AsChangeset for
            &'update $struct_name
        {
            type Target = $table_name::table;
            type Changeset = $changeset_ty;

            fn as_changeset(self) -> Self::Changeset {
                $changeset_body
            }
        }
    )
}

#[allow(unused_imports)]
fn save_changes_impl(
    cx: &mut ExtCtxt,
    span: Span,
    options: &ChangesetOptions,
    model: &Model,
) -> Vec<Option<P<ast::Item>>> {
    let ref struct_name = model.ty;
    let pk = model.primary_key_name();
    let sql_type = cx.path(span, vec![options.table_name, str_to_ident("SqlType")]);
    let table = cx.path(span, vec![options.table_name, str_to_ident("table")]);
    let mut result = Vec::new();
    if let Some(pk) = model.attrs.iter().find(|a| a.column_name == pk) {
        let pk_field = pk.field_name.unwrap();
        if cfg!(feature = "postgres") {
            result.push(quote_item!(cx,
                impl<'a> SaveChangesDsl<
                    ::diesel::pg::PgConnection,
                    $sql_type,
                > for $struct_name {
                    fn save_changes<T>(
                        &self,
                        connection: &::diesel::pg::PgConnection,
                    ) -> ::diesel::QueryResult<T> where
                        T: Queryable<$sql_type, ::diesel::pg::Pg>,
                    {
                        use ::diesel::update;
                        update($table.filter($table.primary_key().eq(&self.$pk_field)))
                            .set(self)
                            .get_result(connection)
                    }
                }
            ));
        }
        if cfg!(feature = "sqlite") {
            result.push(quote_item!(cx,
                impl<'a> SaveChangesDsl<
                    ::diesel::sqlite::SqliteConnection,
                    $sql_type,
                > for $struct_name {
                    fn save_changes<T>(
                        &self,
                        connection: &::diesel::sqlite::SqliteConnection,
                    ) -> ::diesel::QueryResult<T> where
                        T: Queryable<$sql_type, ::diesel::sqlite::Sqlite>,
                    {
                        use ::diesel::update;
                        try!(update($table.filter($table.primary_key().eq(&self.$pk_field)))
                            .set(self)
                            .execute(connection));
                        $table.find(&self.$pk_field).first(connection)
                    }
                }
            ));
        }
    }
    result
}

fn changeset_ty(
    cx: &ExtCtxt,
    span: Span,
    options: &ChangesetOptions,
    attr: &Attr,
) -> P<ast::Ty> {
    let column = cx.path(span, vec![options.table_name, attr.column_name]);
    match (options.treat_none_as_null, ty_param_of_option(&attr.ty)) {
        (false, Some(ty)) => {
            let inner_ty = inner_changeset_ty(cx, column, &ty);
            quote_ty!(cx, Option<$inner_ty>)
        }
        _ => inner_changeset_ty(cx, column, &attr.ty),
    }
}

fn inner_changeset_ty(
    cx: &ExtCtxt,
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
    cx: &ExtCtxt,
    span: Span,
    options: &ChangesetOptions,
    attr: &Attr,
) -> P<ast::Expr> {
    let column = cx.path(span, vec![options.table_name, attr.column_name]);
    let field_name = &attr.field_name.unwrap();
    if !options.treat_none_as_null && is_option_ty(&attr.ty) {
        quote_expr!(cx, self.$field_name.as_ref().map(|f| $column.eq(f)))
    } else {
        quote_expr!(cx, $column.eq(&self.$field_name))
    }
}

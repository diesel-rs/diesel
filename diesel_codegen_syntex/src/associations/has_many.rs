use syntax::ast::MetaItem;
use syntax::codemap::Span;
use syntax::ext::base::{Annotatable, ExtCtxt};

use super::{parse_association_options, to_foreign_key};

pub fn expand_has_many(
    cx: &mut ExtCtxt,
    span: Span,
    meta_item: &MetaItem,
    annotatable: &Annotatable,
    push: &mut FnMut(Annotatable)
) {
    let options = parse_association_options("has_many", cx, span, meta_item, annotatable);
    if let Some((model, options)) = options {
        let parent_table_name = model.table_name();
        let child_table_name = options.name;
        let foreign_key_name = to_foreign_key(&model.name.name.as_str());
        let fields = model.field_tokens_for_stable_macro(cx);
        push(Annotatable::Item(quote_item!(cx, HasMany! {
            (
                parent_table_name = $parent_table_name,
                child_table = $child_table_name::table,
                foreign_key = $child_table_name::$foreign_key_name,
            ),
            fields = [$fields],
        }).unwrap()));
    }
}

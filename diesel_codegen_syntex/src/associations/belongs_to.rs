use syntax::ast::MetaItem;
use syntax::codemap::Span;
use syntax::ext::base::{Annotatable, ExtCtxt};

use super::{parse_association_options, to_foreign_key};

#[allow(unused_imports)]
pub fn expand_belongs_to(
    cx: &mut ExtCtxt,
    span: Span,
    meta_item: &MetaItem,
    annotatable: &Annotatable,
    push: &mut FnMut(Annotatable),
) {
    let options = parse_association_options("belongs_to", cx, span, meta_item, annotatable);
    if let Some((model, options)) = options {
        let parent_struct = options.name;
        let struct_name = model.name;

        let foreign_key_name = options.foreign_key_name.unwrap_or_else(||
            to_foreign_key(&parent_struct.name.as_str()));
        let child_table_name = model.table_name();
        let fields = model.field_tokens_for_stable_macro(cx);
        push(Annotatable::Item(quote_item!(cx, BelongsTo! {
            (
                struct_name = $struct_name,
                parent_struct = $parent_struct,
                foreign_key_name = $foreign_key_name,
                child_table_name = $child_table_name,
            ),
            fields = [$fields],
        }).unwrap()));
    }
}

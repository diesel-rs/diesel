#![deny(warnings)]

#[macro_use] extern crate diesel;
extern crate diesel_codegen_shared;

extern crate syntex;
extern crate syntex_syntax as syntax;

include!(concat!(env!("OUT_DIR"), "/lib.rs"));

mod util;

use std::path::Path;

pub fn expand(input: &Path, output: &Path) -> Result<(), syntex::Error> {
    let mut reg = syntex::Registry::new();
    reg.add_attr("feature(custom_derive)");
    reg.add_attr("feature(custom_attribute)");

    reg.add_decorator("derive_AsChangeset", update::expand_derive_as_changeset);
    reg.add_decorator("derive_Associations", associations::expand_derive_associations);
    reg.add_decorator("derive_Identifiable", identifiable::expand_derive_identifiable);
    reg.add_decorator("derive_Insertable", insertable::expand_derive_insertable);
    reg.add_decorator("derive_Queryable", queryable::expand_derive_queryable);
    reg.add_macro("embed_migrations", migrations::expand_embed_migrations);
    reg.add_macro("infer_table_from_schema", schema_inference::expand_load_table);
    reg.add_macro("infer_schema", schema_inference::expand_infer_schema);

    reg.add_post_expansion_pass(util::strip_attributes);
    reg.expand("", input, output)
}

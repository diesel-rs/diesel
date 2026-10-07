use crate::pg::PgValue;
use crate::types::enum_::{EnumMapping, EnumTypeMapping, EnumVariant};
use std::io::Write;

impl EnumMapping<crate::pg::Pg> for EnumTypeMapping {
    fn map_to_database_value<'b>(
        output: &mut crate::serialize::Output<'b, '_, crate::pg::Pg>,
        variant: &'static EnumVariant,
    ) -> crate::serialize::Result {
        output.write_all(variant.sql_name.as_bytes())?;
        Ok(crate::serialize::IsNull::No)
    }

    fn map_from_database_value(
        raw: PgValue<'_>,
        type_name: &'static str,
        variants: &'static [EnumVariant],
    ) -> crate::deserialize::Result<usize> {
        let name = raw.as_bytes();
        variants
            .iter()
            .position(|v| v.sql_name.as_bytes() == name)
            .ok_or_else(|| {
                format!(
                    "Invalid enum variant `{}` for `{type_name}`. Allowed variants are {}",
                    String::from_utf8_lossy(name),
                    variants
                        .iter()
                        .map(|v| format!("`{}`", v.sql_name))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
                .into()
            })
    }
}

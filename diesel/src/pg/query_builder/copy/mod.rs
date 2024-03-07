use crate::pg::Pg;
use crate::query_builder::nodes::StaticQueryFragment;
use crate::query_builder::ColumnList;
use crate::query_builder::QueryFragment;
use crate::sql_types::SqlType;
use crate::Expression;
use crate::{Column, Table};

pub(crate) mod copy_in;
pub(crate) mod copy_out;

pub(crate) use self::copy_in::{CopyInExpression, InternalCopyInQuery};
pub(crate) use self::copy_out::CopyTo;

pub use self::copy_in::CopyHeader;

const COPY_MAGIC_HEADER: [u8; 11] = [
    0x50, 0x47, 0x43, 0x4F, 0x50, 0x59, 0x0A, 0xFF, 0x0D, 0x0A, 0x00,
];

#[derive(Default, Debug, Copy, Clone)]
pub enum CopyFormat {
    #[default]
    Text,
    Csv,
    Binary,
}

impl CopyFormat {
    fn to_sql_format(&self) -> &'static str {
        match self {
            CopyFormat::Text => "text",
            CopyFormat::Csv => "csv",
            CopyFormat::Binary => "binary",
        }
    }
}

#[derive(Default, Debug)]
struct CommonOptions {
    format: Option<CopyFormat>,
    freeze: Option<bool>,
    delimiter: Option<char>,
    null: Option<String>,
    quote: Option<char>,
    escape: Option<char>,
}

impl CommonOptions {
    fn any_set(&self) -> bool {
        self.format.is_some()
            || self.freeze.is_some()
            || self.delimiter.is_some()
            || self.null.is_some()
            || self.quote.is_some()
            || self.escape.is_some()
    }

    fn walk_ast<'b>(
        &'b self,
        mut pass: crate::query_builder::AstPass<'_, 'b, Pg>,
        comma: &mut &'static str,
    ) -> crate::QueryResult<()> {
        if let Some(format) = self.format {
            pass.push_sql(*comma);
            *comma = ", ";
            pass.push_sql("FORMAT ");
            pass.push_sql(format.to_sql_format());
        }
        if let Some(freeze) = self.freeze {
            pass.push_sql(&format!("{comma}FREEZE {}", freeze as u8));
            *comma = ", ";
        }
        if let Some(delimiter) = self.delimiter {
            pass.push_sql(&format!("{comma}DELIMITER '{delimiter}'"));
            *comma = ", ";
        }
        if let Some(ref null) = self.null {
            pass.push_sql(*comma);
            *comma = ", ";
            pass.push_sql("NULL '");
            // we cannot use binds here :(
            pass.push_sql(null);
            pass.push_sql("'");
        }
        if let Some(quote) = self.quote {
            pass.push_sql(&format!("{comma}QUOTE '{quote}'"));
            *comma = ", ";
        }
        if let Some(escape) = self.escape {
            pass.push_sql(&format!("{comma}ESCAPE '{escape}'"));
            *comma = ", ";
        }
        Ok(())
    }
}

pub trait CopyTarget {
    type Table: Table;
    type SqlType: SqlType;

    fn walk_target<'b>(pass: crate::query_builder::AstPass<'_, 'b, Pg>) -> crate::QueryResult<()>;
}

impl<T> CopyTarget for T
where
    T: Table + StaticQueryFragment,
    T::SqlType: SqlType,
    T::AllColumns: ColumnList,
    T::Component: QueryFragment<Pg>,
{
    type Table = Self;
    type SqlType = T::SqlType;

    fn walk_target<'b>(
        mut pass: crate::query_builder::AstPass<'_, 'b, Pg>,
    ) -> crate::QueryResult<()> {
        T::STATIC_COMPONENT.walk_ast(pass.reborrow())?;
        pass.push_sql("(");
        T::all_columns().walk_ast(pass.reborrow())?;
        pass.push_sql(")");
        Ok(())
    }
}

macro_rules! copy_target_for_columns {
    ($(
        $Tuple:tt {
            $(($idx:tt) -> $T:ident, $ST:ident, $TT:ident,)+
        }
    )+) => {
        $(
            impl<T, $($ST,)*> CopyTarget for ($($ST,)*)
            where
                $($ST: Column<Table = T>,)*
                ($(<$ST as Expression>::SqlType,)*): SqlType,
                T: Table + StaticQueryFragment,
                T::Component: QueryFragment<Pg>,
                Self: ColumnList + Default,
            {
                type Table = T;
                type SqlType = crate::dsl::SqlTypeOf<Self>;

                fn walk_target<'b>(
                    mut pass: crate::query_builder::AstPass<'_, 'b, Pg>,
                ) -> crate::QueryResult<()> {
                    T::STATIC_COMPONENT.walk_ast(pass.reborrow())?;
                    pass.push_sql("(");
                    <Self as ColumnList>::walk_ast(&Self::default(), pass.reborrow())?;
                    pass.push_sql(")");
                    Ok(())
                }
            }
        )*
    }
}

diesel_derives::__diesel_for_each_tuple!(copy_target_for_columns);

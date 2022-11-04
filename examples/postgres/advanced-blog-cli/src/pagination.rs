use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::query_builder::*;
use diesel::query_dsl::methods::LoadQuery;
use diesel::sql_types::BigInt;
use std::fmt::Debug;

pub trait Paginate: Sized {
    fn paginate(self, page: Option<i64>) -> Paginated<Self>;
}

impl<T> Paginate for T {
    fn paginate(self, page: Option<i64>) -> Paginated<Self> {
        let offset = if page.is_some() { (page.clone().unwrap() - 1) * DEFAULT_PER_PAGE } else { -1 };
        Paginated {
            query: self,
            some_per_page: Some(DEFAULT_PER_PAGE),
            per_page: DEFAULT_PER_PAGE,
            page,
            offset,
        }
    }
}

const DEFAULT_PER_PAGE: i64 = 10;

#[derive(Debug, Clone, Copy, QueryId)]
pub struct Paginated<T> {
    query: T,
    page: Option<i64>,
    some_per_page: Option<i64>,
    per_page: i64,
    offset: i64,
}

impl<T> Paginated<T> {
    pub fn per_page(self, some_per_page: Option<i64>) -> Self {
        let per_page = if some_per_page.is_some() { some_per_page.clone().unwrap() } else { -1 };
        let offset = if some_per_page.is_some() && self.page.is_some() { (self.page.clone().unwrap() - 1) * some_per_page.clone().unwrap() } else { -1 };

        Paginated {
            some_per_page,
            per_page,
            offset,
            ..self
        }
    }

    pub fn load_and_count_pages<'a, U>(
        self,
        conn: &mut PgConnection,
    ) -> QueryResult<(Vec<U>, i64, i64)>
        where
            Self: LoadQuery<'a, PgConnection, (U, i64)>,
    {
        let some_page = self.page.clone();
        let some_per_page = self.some_per_page.clone();

        let results = self.load::<(U, i64)>(conn);

        let unwrapped_results = results?;

        if some_page.is_some() && some_per_page.is_some() {
            let per_page = some_per_page.unwrap();
            let total = unwrapped_results.get(0).map(|x| x.1).unwrap_or(0);
            let records = unwrapped_results.into_iter().map(|x| x.0).collect();
            let total_pages = (total as f64 / per_page as f64).ceil() as i64;
            Ok((records, total_pages, total))
        } else {
            let total = unwrapped_results.get(0).map(|x| x.1).unwrap_or(0);
            let records = unwrapped_results.into_iter().map(|x| x.0).collect();
            Ok((records, 1, total))
        }
    }
}

impl<T: Query> Query for Paginated<T> {
    type SqlType = (T::SqlType, BigInt);
}

impl<T> RunQueryDsl<PgConnection> for Paginated<T> {}

impl<T> QueryFragment<Pg> for Paginated<T>
    where
        T: QueryFragment<Pg>,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, Pg>) -> QueryResult<()> {
        if self.page.is_some() && self.some_per_page.is_some() {
            out.push_sql("SELECT *, COUNT(*) OVER () FROM (");
            self.query.walk_ast(out.reborrow())?;
            out.push_sql(") t LIMIT ");
            out.push_bind_param::<BigInt, _>(&self.per_page)?;
            out.push_sql(" OFFSET ");
            out.push_bind_param::<BigInt, _>(&self.offset)?;
        } else {
            out.push_sql("SELECT *, COUNT(*) OVER () FROM (");
            self.query.walk_ast(out.reborrow())?;
            out.push_sql(") t");
        }
        Ok(())
    }
}

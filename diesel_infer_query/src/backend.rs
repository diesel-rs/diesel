// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use sqlparser::dialect::{Dialect, MySqlDialect, PostgreSqlDialect, SQLiteDialect};

/// The database a view definition comes from
///
/// It decides how the definition is parsed, and the rules that depend on how the
/// database evaluates an expression, like whether a cast can return `NULL`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Backend {
    /// PostgreSQL
    Pg,
    /// SQLite
    Sqlite,
    /// MySQL
    Mysql,
    /// MariaDB
    Mariadb,
}

impl Backend {
    pub(crate) fn dialect(self) -> &'static dyn Dialect {
        match self {
            Backend::Pg => &PostgreSqlDialect {},
            Backend::Sqlite => &SQLiteDialect {},
            Backend::Mysql | Backend::Mariadb => &MySqlDialect {},
        }
    }
}

Diesel CLI
==========

Diesel CLI is a tool that aids in managing your database schema. Migrations are
bi-directional changes to your database that get applied sequentially.

Getting Started
---------------

```shell
cargo install diesel_cli
diesel setup --database-url='postgres://localhost/my_db'
diesel migration generate create_users_table
```

You'll see that a `migrations/` directory was generated for you (by the setup
command), and two sql files were generated,
`migrations/{current_timestamp}_create_users_table/up.sql` and
`migrations/{current_timestamp}_create_users_table/down.sql`. You should edit
these files to show how to update your schema, and how to undo that change.

```sql
-- up.sql
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    name VARCHAR NOT NULL,
    favorite_color VARCHAR
);
```

```sql
-- down.sql
DROP TABLE USERS;
```

You can then run your new migration by running `diesel migration run`. Make
sure that you set the `DATABASE_URL` environment variable first, or pass it
directly by doing `diesel migration run
--database-url="postgres://localhost/your_database"` Alternatively, you can
call [`diesel::migrations::run_pending_migrations`][pending-migrations] from
`build.rs`.

Diesel will automatically keep track of which migrations have already been run,
ensuring that they're never run twice.

## Commands
## `diesel setup`
Searches for a `migrations/` directory, and if it can't find one, creates one
in the same directory as the first `Cargo.toml` it finds.  It then tries to
connect to the provided DATABASE_URL, and will create the given database if it
cannot connect to it. Finally it will create diesel's internal table for
tracking which migrations have been run, and run any existing migrations if the
internal table did not previously exist.

## `diesel database`
#### `database setup`
Tries to connect to the provided DATABASE_URL, and will create the given
database if it cannot connect to it.  It then creates diesel's internal
migrations tracking table if it needs to be created, and runs any pending
migrations if it created the internal table.

#### `database reset`
Drops the database specified in your DATABASE_URL if it can, and then runs
`database database setup`.

## `diesel migration`
#### `migration generate`
Takes the name of your migration as an argument, and will create a migration
directory with `migrations/` in the format of
`migrations/{current_timestamp}_{migration_name}`.  It will also generate
`up.sql` and `down.sql` files, for running your migration up and down
respectively.

#### `migration run`
Runs all pending migrations, as determined by diesel's internal schema table.

#### `migration revert`
Runs the `down.sql` for the most recent migration.

#### `migration redo`
Runs the `down.sql` and then the `up.sql` for the most recent migration.

[pending-migrations]: http://sgrif.github.io/diesel/diesel/migrations/fn.run_pending_migrations.html

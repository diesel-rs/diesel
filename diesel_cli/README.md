Diesel CLI
==========

Diesel CLI is a tool that aids in managing your database schema. Migrations are
bi-directional changes to your database that get applied sequentially.

Getting Started
---------------

```shell
cargo install diesel_cli
mkdir migrations
diesel migration generate create_users_table
```

You'll see that two files were generated for you,
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

You can then run your new migration by running `diesel migration run`. Make sure
that you set the `DATABASE_URL` environment variable first, or pass it directly
by doing `diesel migration run --database-url="postgres://localhost/your_database"`
Alternatively, you can call
[`diesel::migrations::run_pending_migrations`][pending-migrations] from
`build.rs`.

Diesel will automatically keep track of which migrations have already been run,
ensuring that they're never run twice.

[pending-migrations]: http://sgrif.github.io/diesel/diesel/migrations/fn.run_pending_migrations.html

If you ever need to revert or make changes to your migrations, the commands
`diesel migration revert` and `diesel migration redo`. Type `diesel migration
--help` for more information.

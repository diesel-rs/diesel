# `rs-diesel-sqlite`

Diesel's `Getting Started` guide using SQLite instead of Postgresql

## Usage

```
$ echo "DATABASE_URL=file:test.db" > .env
$ diesel migration run

$ cargo run --bin show_posts

$ cargo run --bin write_post
# write your post
```

# `rs-diesel-sqlite`

Diesel's `Getting Started` guide using SQLite instead of Postgresql

## Usage

```
$ echo "DATABASE_URL=file:test.db" > .env
$ diesel migration run

$ cargo run --bin show_posts

$ cargo run --bin write_post
# write your post

$ cargo run --bin publish_post 1

$ cargo run --bin show_posts
# your post will be printed here

$ cargo run --bin delete_post 1

$ cargo run --bin show_posts
# observe that no posts are shown
```

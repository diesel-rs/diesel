## 1. Avoid Namespace Conflicts with `#[derive(Insertable)]`

When using `#[derive(Insertable)]`, avoid importing multiple table DSL modules (like `users::dsl::*`, `posts::dsl::*`) into the same scope.  
This can lead to naming conflicts because Diesel expands macros that may reuse column identifiers from different tables.

**✅ Correct Example**
```rust
use crate::schema::users;

#[derive(Insertable)]
#[table_name = "users"]
struct NewUser<'a> {
    name: &'a str,
}
use crate::schema::{users::dsl::*, posts::dsl::*}; // Causes naming conflicts

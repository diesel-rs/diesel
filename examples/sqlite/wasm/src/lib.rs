pub mod models;
pub mod schema;

use std::sync::Mutex;
use std::sync::Once;

use crate::models::{NewPost, Post};
use diesel::prelude::*;
use diesel_migrations::EmbeddedMigrations;
use diesel_migrations::MigrationHarness;
use diesel_migrations::embed_migrations;
use wasm_bindgen::prelude::*;

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

#[wasm_bindgen]
extern "C" {
    // Use `js_namespace` here to bind `console.log(..)` instead of just
    // `log(..)`
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

// Next let's define a macro that's like `println!`, only it works for
// `console.log`. Note that `println!` doesn't actually work on the Wasm target
// because the standard library currently just eats all output. To get
// `println!`-like behavior in your app you'll likely want a macro like this.
macro_rules! console_log {
    // Note that this is using the `log` function imported above during
    // `bare_bones`
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

static VFS: Mutex<(i32, Once)> = Mutex::new((0, Once::new()));

pub fn establish_connection() -> SqliteConnection {
    let (vfs, once) = &*VFS.lock().unwrap();
    let url = match vfs {
        0 => "post.db",
        1 => "file:post.db?vfs=opfs-sahpool",
        2 => "file:post.db?vfs=relaxed-idb",
        _ => unreachable!(),
    };
    let mut conn =
        SqliteConnection::establish(url).unwrap_or_else(|_| panic!("Error connecting to post.db"));
    once.call_once(|| {
        conn.run_pending_migrations(MIGRATIONS).unwrap();
    });
    conn
}

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
#[wasm_bindgen(js_name = installOpfsSahpool)]
pub async fn install_opfs_sahpool() {
    use sqlite_wasm_vfs::sahpool::{OpfsSAHPoolCfg, install};
    install::<sqlite_wasm_rs::WasmOsCallback>(&OpfsSAHPoolCfg::default(), false)
        .await
        .unwrap();
}

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
#[wasm_bindgen(js_name = installRelaxedIdb)]
pub async fn install_relaxed_idb() {
    use sqlite_wasm_vfs::relaxed_idb::{RelaxedIdbCfg, install};
    install::<sqlite_wasm_rs::WasmOsCallback>(&RelaxedIdbCfg::default(), false)
        .await
        .unwrap();
}

#[wasm_bindgen(js_name = switchVfs)]
pub fn switch_vfs(id: i32) {
    *VFS.lock().unwrap() = (id, Once::new());
}

#[wasm_bindgen(js_name = createPost)]
pub fn create_post(title: &str, body: &str) -> JsValue {
    use crate::schema::posts;

    let new_post = NewPost { title, body };

    let post = diesel::insert_into(posts::table)
        .values(&new_post)
        .returning(Post::as_returning())
        .get_result(&mut establish_connection())
        .expect("Error saving new post");

    serde_wasm_bindgen::to_value(&post).unwrap()
}

#[wasm_bindgen(js_name = deletePost)]
pub fn delete_post(pattern: &str) {
    let connection = &mut establish_connection();
    let num_deleted = diesel::delete(
        schema::posts::dsl::posts.filter(schema::posts::title.like(pattern.to_string())),
    )
    .execute(connection)
    .expect("Error deleting posts");

    console_log!("Deleted {num_deleted} posts");
}

#[wasm_bindgen(js_name = getPost)]
pub fn get_post(post_id: i32) -> JsValue {
    use schema::posts::dsl::posts;

    let connection = &mut establish_connection();

    let post = posts
        .find(post_id)
        .select(Post::as_select())
        .first(connection)
        .optional(); // This allows for returning an Option<Post>, otherwise it will throw an error

    match &post {
        Ok(Some(post)) => console_log!("Post with id: {} has a title: {}", post.id, post.title),
        Ok(None) => console_log!("Unable to find post {}", post_id),
        Err(_) => console_log!("An error occurred while fetching post {}", post_id),
    }
    serde_wasm_bindgen::to_value(&post.ok().flatten()).unwrap()
}

#[wasm_bindgen(js_name = publishPost)]
pub fn publish_post(id: i32) {
    let connection = &mut establish_connection();

    let post = diesel::update(schema::posts::dsl::posts.find(id))
        .set(schema::posts::dsl::published.eq(true))
        .returning(Post::as_returning())
        .get_result(connection)
        .unwrap();

    console_log!("Published post {}", post.title);
}

#[wasm_bindgen(js_name = showPosts)]
pub fn show_posts() -> Vec<JsValue> {
    let connection = &mut establish_connection();
    let results = schema::posts::dsl::posts
        .filter(schema::posts::dsl::published.eq(true))
        .limit(5)
        .select(Post::as_select())
        .load(connection)
        .expect("Error loading posts");

    console_log!("Displaying {} posts", results.len());
    for post in &results {
        console_log!("{}", post.title);
        console_log!("----------\n");
        console_log!("{}", post.body);
    }

    results
        .into_iter()
        .map(|x| serde_wasm_bindgen::to_value(&x).unwrap())
        .collect()
}

mod auth;
mod cli;
mod comment;
mod editor;
mod pagination;
mod post;
mod schema;

#[cfg(test)]
mod test_helpers;

use diesel::prelude::*;
use structopt::StructOpt;

use std::error::Error;

use self::cli::Cli;
use self::pagination::*;
use self::post::*;
use self::schema::*;

fn main() {
    let matches = Cli::from_args();

    let database_url = dotenvy::var("DATABASE_URL").expect("DATABASE_URL must be set");

    handle_error(run_cli(&database_url, matches));
}

fn run_cli(database_url: &str, cli: Cli) -> Result<(), Box<dyn Error>> {
    let conn = &mut PgConnection::establish(database_url)?;

    match cli {
        Cli::AllPosts { page, per_page } => {
            use auth::User;
            use comment::*;

            let mut query = posts::table
                .order(posts::published_at.desc())
                .filter(posts::published_at.is_not_null())
                .inner_join(users::table)
                .select((posts::all_columns, (users::id, users::username)))
                .paginate(page);

            if let Some(per_page) = per_page {
                use std::cmp::min;
                query = query.per_page(min(per_page, 25));
            }

            let (posts_with_user, total_pages) =
                query.load_and_count_pages::<(Post, User)>(conn)?;
            let (posts, post_users): (Vec<_>, Vec<_>) = posts_with_user.into_iter().unzip();

            let comments = Comment::belonging_to(&posts)
                .inner_join(users::table)
                .select((comments::all_columns, (users::id, users::username)))
                .load::<(Comment, User)>(conn)?
                .grouped_by(&posts);

            let to_display = posts.into_iter().zip(post_users).zip(comments);
            for ((post, user), comments) in to_display {
                post::render(&post, &user, &comments);
            }

            println!("Page {} of {}", page, total_pages);
        }
        Cli::CreatePost { title } => {
            let user = current_user(conn)?;
            let body = editor::edit_string("")?;
            let id = diesel::insert_into(posts::table)
                .values((
                    posts::user_id.eq(user.id),
                    posts::title.eq(title),
                    posts::body.eq(body),
                ))
                .returning(posts::id)
                .get_result::<i32>(conn)?;
            println!("Successfully created post with id {}", id);
        }
        Cli::EditPost { post_id, publish } => {
            use diesel::dsl::now;
            use post::Status::*;
            use schema::posts::dsl::*;

            let user = current_user(conn)?;
            let post = Post::belonging_to(&user)
                .find(post_id)
                .first::<Post>(conn)?;
            let new_body = editor::edit_string(&post.body)?;

            let updated_status = match post.status {
                Draft if publish => Some(published_at.eq(now.nullable())),
                _ => None,
            };

            diesel::update(&post)
                .set((body.eq(new_body), updated_status))
                .execute(conn)?;
        }
        Cli::AddComment {
            post_id: given_post_id,
        } => {
            use schema::comments::dsl::*;

            let inserted = diesel::insert_into(comments)
                .values((
                    user_id.eq(current_user(conn)?.id),
                    post_id.eq(given_post_id),
                    body.eq(editor::edit_string("")?),
                ))
                .returning(id)
                .get_result::<i32>(conn)?;
            println!("Created comment with ID {}", inserted);
        }
        Cli::EditComment { comment_id } => {
            use comment::Comment;
            use schema::comments::dsl::*;

            let user = current_user(conn)?;

            let comment = Comment::belonging_to(&user)
                .find(comment_id)
                .first::<Comment>(conn)?;

            diesel::update(comments)
                .set(body.eq(editor::edit_string(&comment.body)?))
                .execute(conn)?;
        }
        Cli::MyComments { page, per_page } => {
            use comment::Comment;

            let user = current_user(conn)?;

            let mut query = Comment::belonging_to(&user)
                .order(comments::created_at.desc())
                .inner_join(posts::table)
                .select((comments::all_columns, posts::title))
                .paginate(page);

            if let Some(per_page) = per_page {
                use std::cmp::min;
                query = query.per_page(min(per_page, 25));
            }

            let (comments_and_post_title, total_pages) =
                query.load_and_count_pages::<(Comment, String)>(conn)?;
            comment::render(&comments_and_post_title);
            println!("Page {} of {}", page, total_pages);
        }
        Cli::Register => {
            register_user(conn)?;
        }
    }
    Ok(())
}

fn current_user(conn: &mut PgConnection) -> Result<auth::User, Box<dyn Error>> {
    match auth::current_user_from_env(conn) {
        Ok(Some(user)) => Ok(user),
        Ok(None) => Err("No user found with the given username".into()),
        Err(e) => Err(convert_auth_error(e)),
    }
}

fn register_user(conn: &mut PgConnection) -> Result<(), Box<dyn Error>> {
    use auth::AuthenticationError as Auth;
    use diesel::result::DatabaseErrorKind::UniqueViolation;
    use diesel::result::Error::DatabaseError;

    match auth::register_user_from_env(conn) {
        Ok(_) => Ok(()),
        Err(Auth::DatabaseError(DatabaseError(UniqueViolation, _))) => {
            Err("A user with that name already exists".into())
        }
        Err(e) => Err(convert_auth_error(e)),
    }
}

fn convert_auth_error(err: auth::AuthenticationError) -> Box<dyn Error> {
    use auth::AuthenticationError::*;

    match err {
        IncorrectPassword => "The password given does not match our records".into(),
        NoUsernameSet => {
            "No username given. You need to set the BLOG_USERNAME environment variable.".into()
        }
        NoPasswordSet => {
            "No password given. You need to set the BLOG_PASSWORD environment variable.".into()
        }
        EnvironmentError(e) => e.into(),
        BcryptError(e) => e.into(),
        DatabaseError(e) => e.into(),
    }
}

fn handle_error<T>(res: Result<T, Box<dyn Error>>) -> T {
    match res {
        Ok(x) => x,
        Err(e) => print_error_and_exit(&*e),
    }
}

fn print_error_and_exit(err: &dyn Error) -> ! {
    use std::process::exit;
    eprintln!("An unexpected error occurred: {}", err);
    exit(1);
}

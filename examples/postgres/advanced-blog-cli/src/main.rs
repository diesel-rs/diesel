#![deny(warnings)]

extern crate bcrypt;
extern crate chrono;
#[macro_use]
extern crate clap;
#[macro_use]
extern crate diesel;
extern crate dotenv;

#[cfg(test)]
#[macro_use]
extern crate assert_matches;
#[cfg(test)]
#[macro_use]
extern crate lazy_static;

mod auth;
mod cli;
mod comment;
mod editor;
mod pagination;
mod post;
mod schema;

#[cfg(test)]
mod test_helpers;

use clap::ArgMatches;
use diesel::prelude::*;
use std::error::Error;

use self::schema::*;
use self::post::*;
use self::pagination::*;

fn main() {
    let matches = cli::build_cli().get_matches();
    let database_url = dotenv::var("DATABASE_URL").expect("DATABASE_URL must be set");

    handle_error(run_cli(&database_url, &matches));
}

fn run_cli(database_url: &str, matches: &ArgMatches) -> Result<(), Box<Error>> {
    let conn = PgConnection::establish(database_url)?;

    match matches.subcommand() {
        ("all_posts", Some(args)) => {
            use comment::*;
            use auth::User;

            let (page, per_page) = pagination_args(args)?;
            let mut query = posts::table
                .order(posts::published_at.desc())
                .filter(posts::published_at.is_not_null())
                .inner_join(users::table)
                .select((posts::all_columns, (users::id, users::username)))
                .paginate(page);

            if let Some(per_page) = per_page {
                query = query.per_page(per_page);
            }

            let (posts_with_user, total_pages) = query.load_and_count_pages::<(Post, User)>(&conn)?;
            let (posts, post_users): (Vec<_>, Vec<_>) = posts_with_user.into_iter().unzip();

            let comments = Comment::belonging_to(&posts)
                .inner_join(users::table)
                .select((comments::all_columns, (users::id, users::username)))
                .load::<(Comment, User)>(&conn)?
                .grouped_by(&posts);

            let to_display = posts.into_iter().zip(post_users).zip(comments);
            for ((post, user), comments) in to_display {
                post::render(&post, &user, &comments);
            }

            println!("Page {} of {}", page, total_pages);
        }

        ("create_post", Some(args)) => {
            let title = args.value_of("TITLE").unwrap();
            let user = current_user(&conn)?;
            let body = editor::edit_string("")?;
            let id = diesel::insert_into(posts::table)
                .values((
                    posts::user_id.eq(user.id),
                    posts::title.eq(title),
                    posts::body.eq(body),
                ))
                .returning(posts::id)
                .get_result::<i32>(&conn)?;
            println!("Successfully created post with id {}", id);
        }

        ("edit_post", Some(args)) => {
            use schema::posts::dsl::*;
            use post::Status::*;
            use diesel::dsl::now;

            let post_id = value_t!(args, "POST_ID", i32)?;
            let user = current_user(&conn)?;
            let post = Post::belonging_to(&user)
                .find(post_id)
                .first::<Post>(&conn)?;
            let new_body = editor::edit_string(&post.body)?;

            let updated_status = match post.status {
                Draft if args.is_present("PUBLISH") => Some(published_at.eq(now.nullable())),
                _ => None,
            };

            diesel::update(&post)
                .set((body.eq(new_body), updated_status))
                .execute(&conn)?;
        }

        ("add_comment", Some(args)) => {
            use schema::comments::dsl::*;

            let inserted = diesel::insert_into(comments)
                .values((
                    user_id.eq(current_user(&conn)?.id),
                    post_id.eq(value_t!(args, "POST_ID", i32)?),
                    body.eq(editor::edit_string("")?),
                ))
                .returning(id)
                .get_result::<i32>(&conn)?;
            println!("Created comment with ID {}", inserted);
        }

        ("edit_comment", Some(args)) => {
            use schema::comments::dsl::*;
            use comment::Comment;

            let user = current_user(&conn)?;

            let comment = Comment::belonging_to(&user)
                .find(value_t!(args, "COMMENT_ID", i32)?)
                .first::<Comment>(&conn)?;

            diesel::update(comments)
                .set(body.eq(editor::edit_string(&comment.body)?))
                .execute(&conn)?;
        }

        ("my_comments", Some(args)) => {
            use comment::Comment;

            let (page, per_page) = pagination_args(args)?;
            let user = current_user(&conn)?;

            let mut query = Comment::belonging_to(&user)
                .order(comments::created_at.desc())
                .inner_join(posts::table)
                .select((comments::all_columns, posts::title))
                .paginate(page);

            if let Some(per_page) = per_page {
                query = query.per_page(per_page);
            }

            let (comments_and_post_title, total_pages) =
                query.load_and_count_pages::<(Comment, String)>(&conn)?;
            comment::render(&comments_and_post_title);
            println!("Page {} of {}", page, total_pages);
        }

        ("register", Some(_)) => register_user(&conn)?,

        _ => unreachable!(),
    }
    Ok(())
}

fn pagination_args(args: &ArgMatches) -> Result<(i64, Option<i64>), Box<Error>> {
    use std::cmp::min;

    let page = args.value_of("PAGE").unwrap_or("1").parse()?;

    if let Some(per_page) = args.value_of("PER_PAGE") {
        let per_page = min(per_page.parse()?, 25);
        Ok((page, Some(per_page)))
    } else {
        Ok((page, None))
    }
}

fn current_user(conn: &PgConnection) -> Result<auth::User, Box<Error>> {
    match auth::current_user_from_env(conn) {
        Ok(Some(user)) => Ok(user),
        Ok(None) => Err("No user found with the given username".into()),
        Err(e) => Err(convert_auth_error(e)),
    }
}

fn register_user(conn: &PgConnection) -> Result<(), Box<Error>> {
    use auth::AuthenticationError as Auth;
    use diesel::result::Error::DatabaseError;
    use diesel::result::DatabaseErrorKind::UniqueViolation;

    match auth::register_user_from_env(conn) {
        Ok(_) => Ok(()),
        Err(Auth::DatabaseError(DatabaseError(UniqueViolation, _))) => {
            Err("A user with that name already exists".into())
        }
        Err(e) => Err(convert_auth_error(e)),
    }
}

fn convert_auth_error(err: auth::AuthenticationError) -> Box<Error> {
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

fn handle_error<T>(res: Result<T, Box<Error>>) -> T {
    match res {
        Ok(x) => x,
        Err(e) => print_error_and_exit(&*e),
    }
}

fn print_error_and_exit(err: &Error) -> ! {
    use std::process::exit;
    eprintln!("An unexpected error occurred: {}", err);
    exit(1);
}

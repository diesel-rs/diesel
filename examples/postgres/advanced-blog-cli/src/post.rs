use auth::User;
use chrono::NaiveDateTime;
use comment::Comment;

use schema::posts;

#[derive(Queryable, Associations, Identifiable)]
#[belongs_to(User)]
pub struct Post {
    pub id: i32,
    pub user_id: i32,
    pub title: String,
    pub body: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub status: Status,
}

pub enum Status {
    Draft,
    Published { at: NaiveDateTime },
}

use diesel::deserialize::Queryable;
use diesel::pg::Pg;
use diesel::sql_types::{Nullable, Timestamp};

impl Queryable<Nullable<Timestamp>, Pg> for Status {
    type Row = Option<NaiveDateTime>;

    fn build(row: Self::Row) -> Self {
        match row {
            Some(at) => Status::Published { at },
            None => Status::Draft,
        }
    }
}

pub fn render(post: &Post, user: &User, comments: &[(Comment, User)]) {
    use self::Status::*;

    println!("{} (id: {})", post.title, post.id);
    println!("By {}", user.username);
    let edited_at = post.updated_at.format("%F %T");
    match post.status {
        Draft => println!("DRAFT (last edited at {})", edited_at),
        Published { at } if at != post.updated_at => {
            let published_at = at.format("%F %T");
            println!(
                "Published at {} (last edited at {})",
                published_at, edited_at
            );
        }
        Published { at } => println!("Published at {}", at.format("%F %T")),
    }
    println!("\n{}", post.body);

    if !comments.is_empty() {
        println!("---------------\n");
        println!("{} Comments\n", comments.len());
        for &(ref comment, ref user) in comments {
            let at = comment.created_at.format("%F %T");
            print!("{} at {}", user.username, at);
            if comment.updated_at != comment.created_at {
                let edited = comment.updated_at.format("%F %T");
                print!(" (last edited {})", edited);
            }
            println!("\n{}", comment.body);
        }
    }

    println!("===============\n");
}

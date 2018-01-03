use chrono::NaiveDateTime;

use auth::User;
use post::Post;
use schema::comments;

#[derive(Queryable, Identifiable, Associations)]
#[belongs_to(User)]
#[belongs_to(Post)]
pub struct Comment {
    pub id: i32,
    pub user_id: i32,
    pub post_id: i32,
    pub body: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

pub fn render(comments_and_post_title: &[(Comment, String)]) {
    for &(ref comment, ref post_title) in comments_and_post_title {
        println!("On post {}", post_title);
        println!(
            "At {} (id: {})",
            comment.updated_at.format("%F %T"),
            comment.id
        );
        println!("{}", comment.body);
        print!("===============\n\n");
    }
}

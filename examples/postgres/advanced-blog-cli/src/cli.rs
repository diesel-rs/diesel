use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "blog",
    after_help = "You can also run `blog SUBCOMMAND -h` to get more information about that subcommand."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Create a new wonderfully delighting blog post
    CreatePost {
        /// The title of your post
        #[arg(long = "title")]
        title: String,
    },
    /// Fine-tune an existing post to reach 100% reader delight
    EditPost {
        /// The id of the post to edit
        post_id: i32,
        /// Announce this piece of literary perfectionisms to the world?
        #[arg(short = 'i')]
        publish: bool,
    },
    /// Get an overview of all your literary accomplishments
    AllPosts {
        /// The page to display
        #[arg(long = "page", default_value = "1")]
        page: i64,
        /// The number of posts to display per page (cannot be larger than 25)
        #[arg(long = "per-page")]
        per_page: Option<i64>,
    },
    /// Quickly add an important remark to a post
    AddComment {
        /// The id of the post to comment on
        post_id: i32,
    },
    /// Edit a comment, e.g. to cite the sources of your facts
    EditComment {
        /// The id of the comment to edit
        comment_id: i32,
    },
    /// See all the instances where you were able to improve a post by adding a comment
    MyComments {
        /// The page to display
        #[arg(long = "page", default_value = "1")]
        page: i64,
        /// The number of comments to display per page (cannot be larger than 25)
        #[arg(long = "per-page")]
        per_page: Option<i64>,
    },
    /// Register as the newest member of this slightly elitist community
    Register,
}

use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(
    name = "blog",
    after_help = "You can also run `blog SUBCOMMAND -h` to get more information about that subcommand."
)]
pub enum Cli {
    /// Create a new wonderfully delighting blog post
    #[structopt(name = "create_post")]
    CreatePost {
        /// The title of your post
        #[structopt(long = "title")]
        title: String,
    },
    /// Fine-tune an existing post to reach 100% reader delight
    #[structopt(name = "edit_post")]
    EditPost {
        /// The id of the post to edit
        post_id: i32,
        /// Announce this piece of literary perfectionisms to the world?
        #[structopt(short = "i")]
        publish: bool,
    },
    /// Get an overview of all your literary accomplishments
    #[structopt(name = "all_posts")]
    AllPosts {
        /// The page to display
        #[structopt(long = "page", default_value = "1")]
        page: i64,
        /// The number of posts to display per page (cannot be larger than 25)
        #[structopt(long = "per-page")]
        per_page: Option<i64>,
    },
    /// Quickly add an important remark to a post
    #[structopt(name = "add_comment")]
    AddComment {
        /// The id of the post to comment on
        post_id: i32,
    },
    /// Edit a comment, e.g. to cite the sources of your facts
    #[structopt(name = "edit_comment")]
    EditComment {
        /// The id of the comment to edit
        comment_id: i32,
    },
    /// See all the instances where you were able to improve a post by adding a comment
    #[structopt(name = "my_comments")]
    MyComments {
        /// The page to display
        #[structopt(long = "page", default_value = "1")]
        page: i64,
        /// The number of comments to display per page (cannot be larger than 25)
        #[structopt(long = "per-page")]
        per_page: Option<i64>,
    },
    /// Register as the newest member of this slightly elitist community
    #[structopt(name = "register")]
    Register,
}

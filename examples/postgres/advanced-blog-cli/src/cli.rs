use clap::{App, AppSettings, Arg, SubCommand};

pub fn build_cli() -> App<'static, 'static> {
    let post_id_arg = Arg::with_name("POST_ID")
        .help("The id of the post to edit")
        .takes_value(true)
        .required(true);
    let page_arg = Arg::with_name("PAGE")
        .long("page")
        .takes_value(true)
        .help("The page to display");
    let per_page_arg = Arg::with_name("PER_PAGE")
        .long("per-page")
        .takes_value(true)
        .help("The number of posts to display per page (cannot be larger than 25)");

    let create_post = SubCommand::with_name("create_post").arg(
        Arg::with_name("TITLE")
            .long("title")
            .help("The title of your post")
            .takes_value(true)
            .required(true),
    );
    let edit_post = SubCommand::with_name("edit_post")
        .arg(post_id_arg.clone())
        .arg(
            Arg::with_name("PUBLISH")
                .long("publish")
                .help("Publish the post after editing"),
        );
    let all_posts = SubCommand::with_name("all_posts")
        .arg(page_arg.clone())
        .arg(per_page_arg.clone());
    let add_comment = SubCommand::with_name("add_comment").arg(post_id_arg.clone());
    let edit_comment = SubCommand::with_name("edit_comment").arg(
        Arg::with_name("COMMENT_ID")
            .help("The id of the comment to edit")
            .takes_value(true)
            .required(true),
    );
    let my_comments = SubCommand::with_name("my_comments")
        .arg(page_arg.clone())
        .arg(per_page_arg.clone());

    App::new("blog")
        .version(env!("CARGO_PKG_VERSION"))
        .setting(AppSettings::VersionlessSubcommands)
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .subcommand(create_post)
        .subcommand(all_posts)
        .subcommand(edit_post)
        .subcommand(add_comment)
        .subcommand(edit_comment)
        .subcommand(my_comments)
        .subcommand(SubCommand::with_name("register"))
        .after_help(
            "You can also run `blog SUBCOMMAND -h` to get more information about that subcommand.",
        )
}

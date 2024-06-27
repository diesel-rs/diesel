use assert_cmd::Command;

#[test]
fn write_post() {
    let _ = Command::cargo_bin("write_post")
        .unwrap()
        .write_stdin("Test Title\ntest text\n1 2 3")
        .assert()
        .append_context("write_post", "")
        .stdout("What would you like your title to be?\n\nOk! Let's write Test Title (Press ".to_owned() + EOF + " when finished)\n\n\nSaved draft Test Title with id 1\n");
    let _ = Command::cargo_bin("show_posts")
        .unwrap()
        .assert()
        .append_context("show_posts", "")
        .stdout("Displaying 0 posts\n");
}

#[cfg(not(windows))]
const EOF: &str = "CTRL+D";

#[cfg(windows)]
const EOF: &str = "CTRL+Z";

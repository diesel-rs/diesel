use assert_cmd::Command;

#[test]
fn write_post() {
    let _ = Command::cargo_bin("show_posts")
        .unwrap()
        .assert()
        .append_context("show_posts", "")
        .stdout("Displaying 0 posts\n");
}


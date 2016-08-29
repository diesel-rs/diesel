use support::project;

#[test]
fn can_generate_bash_completion() {
    let p = project("migration_redo")
        .build();

    let result = p.command("bash-completion").run();

    let expected_last_line = "complete -F _diesel diesel";

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(result.stdout().contains(expected_last_line),
        "Unexpected stdout {}", result.stdout());
}

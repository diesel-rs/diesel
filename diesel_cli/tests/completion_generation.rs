use support::project;

#[test]
fn can_generate_bash_completion() {
    let p = project("migration_redo").build();

    let result = p.command("bash-completion").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
}

#[test]
fn can_generate_zsh_completion() {
    let p = project("migration_redo").build();

    let result = p.command("zsh-completion").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
}

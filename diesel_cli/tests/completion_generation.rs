use crate::support::project;

#[test]
fn can_generate_bash_completion() {
    let p = project("migration_redo").build();

    let result = p.command("completions").arg("bash").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
}

#[test]
fn can_generate_fish_completion() {
    let p = project("migration_redo").build();

    let result = p.command("completions").arg("fish").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
}

#[test]
fn can_generate_zsh_completion() {
    let p = project("migration_redo").build();

    let result = p.command("completions").arg("zsh").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
}

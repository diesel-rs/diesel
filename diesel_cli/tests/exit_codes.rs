use crate::support::project;

#[test]
fn errors_dont_cause_panic() {
    let p = project("errors_dont_panic").build();

    let result = p.command_without_database_url("migration").arg("run").run();

    assert!(!result.is_success());
    assert!(!result.stdout().contains("thread '<main>' panicked at"))
}

#[test]
fn errors_exit_code_is_1() {
    let p = project("errors_exit_1").build();

    let result = p.command_without_database_url("migration").arg("run").run();

    assert_eq!(1, result.code())
}

#[test]
fn successful_run_exits_0() {
    let p = project("successes_exit_0").build();

    let result = p.command("setup").run();

    assert_eq!(0, result.code())
}

use std::env;
use std::path::PathBuf;

fn main() -> ui_test::color_eyre::Result<()> {
    let mut config = ui_test::Config::rustc("tests/fail");
    config.bless_command = Some("BLESS=1 cargo test".into());
    config.filter(
        "   = note: the full name for the type has been written to '.*",
        "",
    );
    let diesel_root_path = PathBuf::from(env!("CARGO_MANIFEST_DIR").to_owned() + "/../")
        .canonicalize()
        .unwrap();
    config.path_filter(&diesel_root_path, "DIESEL");
    if let Ok(target_dir) = env::var("CARGO_TARGET_DIR") {
        // When `CARGO_TARGET_DIR` points outside of the diesel workspace -- e.g.
        // when tests are run from a sandbox that redirects build artifacts to a
        // temporary location -- `path_filter(&diesel_root_path, "DIESEL")` alone
        // doesn't normalize paths emitted by rustc that point at the target
        // directory (such as `<target>/ui/0/.../*.long-type-*.txt`), so they
        // would leak into the blessed `.stderr` files. Map the actual target
        // directory to the canonical in-workspace location so blessing produces
        // the expected output regardless of where cargo places build artifacts.
        //
        // We can't use `Config::path_filter` for this because it canonicalizes
        // the path it's given, takes that path's *parent*, and only registers
        // the parent as the substring to replace (presumably so a caller can
        // pass any concrete file inside the directory they want normalized).
        // Here we already have the directory we want to replace and have no
        // guaranteed-to-exist child file to pass, so we register an explicit
        // (escaped) regex match against the target directory instead.
        if let Ok(target_dir) = PathBuf::from(target_dir).canonicalize() {
            if !target_dir.starts_with(&diesel_root_path) {
                config.filter(
                    &regex::escape(&target_dir.to_string_lossy()),
                    "DIESEL/diesel/diesel_compile_tests/target",
                );
            }
        }
    }
    config.filter("and [0-9]* others", "and N others");
    config.filter(
        "= note: consider using `--verbose` to print the full type name to the console\n",
        "",
    );
    config.filter("\nerror: aborting due to [0-9]* previous error[s]?\n\n", "");
    config.filter(
        "\nerror: aborting due to [0-9]* previous error[s]?; [0-9]* warning[s]? emitted\n\n",
        "",
    );
    config.filter(
        "\n\\s*Some errors have detailed explanations: [E0-9, ]*.",
        "",
    );
    config.filter(
        "\n\\s*For more information about an error, try `rustc --explain E[0-9]*`.",
        "",
    );
    // not sure how well that works on windows
    config.filter(
        "diesel\\/diesel\\/([a-zA-Z_0-9\\/]*)\\.rs:[0-9]*:[0-9]*",
        "diesel/diesel/$1.rs",
    );
    // replace rust standard library paths
    config.filter(
        "\\/rustc\\/[a-z0-9]*\\/library",
        "/rustc/0000000000000000000000000000000000000000/library",
    );
    // replace rust standard library paths
    config.filter("[a-z0-9_\\-\\.\\/]*rust/library", "/rustc/library");
    // that's not perfect as it might
    // as it breaks layout it some cases
    config.filter("[0-9]+ \\|", "LL |");
    // longer type name error name are non-deterministic
    config.filter(
        "long-type-[0-9]+.txt'",
        "long-type-0000000000000000000.txt'",
    );

    config
        .comment_defaults
        .base()
        .compile_flags
        .push("--diagnostic-width=100".into());
    config.comment_defaults.base().set_custom(
        "dependencies",
        ui_test::dependencies::DependencyBuilder {
            crate_manifest_path: PathBuf::from(
                env!("CARGO_MANIFEST_DIR").to_owned() + "/tests/Cargo.toml",
            ),
            program: ui_test::CommandBuilder::cargo(),
            ..Default::default()
        },
    );

    if env::var("BLESS").is_ok() {
        config.output_conflict_handling = ui_test::bless_output_files;
    } else {
        config.output_conflict_handling = ui_test::error_on_output_conflict;
    }

    ui_test::run_tests(config)
}

use cargo_metadata::Metadata;

pub fn get_exclude_for_backend<'a>(
    backend: &str,
    metadata: &'a Metadata,
    wasm: bool,
) -> Vec<&'a str> {
    let examples = metadata.workspace_root.join("examples");
    let backend_examples = examples.join(backend);
    let mut out = metadata
        .workspace_packages()
        .into_iter()
        .filter_map(move |p| {
            if p.manifest_path.starts_with(&examples)
                && !p.manifest_path.starts_with(&backend_examples)
            {
                Some(["--exclude", &p.name])
            } else {
                None
            }
        })
        .flatten()
        .collect::<Vec<_>>();
    if wasm {
        let additional_excludes = [
            // command line tool is not helpful
            // with the wasm32-unknown-unknown target
            "diesel_cli",
            // these pull in libsqlite3-sys
            "getting_started_step_1_sqlite",
            "getting_started_step_2_sqlite",
            "getting_started_step_3_sqlite",
            "all_about_inserts_sqlite",
            "relations_sqlite",
            // needs to be tested in a separate step
            // due to broken cargo workspace feature unification
            "sqlite-wasm-example",
        ];
        out.extend(
            additional_excludes
                .into_iter()
                .flat_map(|v| ["--exclude", v]),
        );
    }
    out
}

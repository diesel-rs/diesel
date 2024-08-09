use cargo_metadata::Metadata;

pub fn get_exclude_for_backend<'a>(backend: &str, metadata: &'a Metadata) -> Vec<&'a str> {
    let examples = metadata.workspace_root.join("examples");
    let backend_examples = examples.join(backend);
    metadata
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
        .collect()
}

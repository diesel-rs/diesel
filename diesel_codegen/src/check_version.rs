use std::process::Command;

#[derive(Deserialize)]
struct Metadata {
    packages: Vec<Package>,
}

#[derive(Deserialize, Debug)]
struct Package {
    name: String,
    version: String,
}

#[allow(warnings)]
pub fn check_version() -> Result<(), ()> {
    let contents = String::from_utf8(
        Command::new("cargo")
            .arg("metadata")
            .output()
            .map_err(|_| ())?
            .stdout,
    ).map_err(|_| ())?;

    let data: Metadata = ::serde_json::from_str(&contents).map_err(|_| ())?;

    let diesel = data.packages
        .into_iter()
        .filter(|p| p.name == "diesel");

    for d in diesel {
        if d.version.starts_with("1.") || d.version.starts_with("0.99") {
            panic!(
                "diesel_codegen was deprecated and removed with\
                 version 0.99. You are trying to use the old codegen with \
                 diesel {}. See the Changelog \
                 (https://github.com/diesel-rs/diesel/blob/master/CHANGELOG.md#changed-6)\
                 for details.",
                d.version
            );
        }
    }
    Ok(())
}

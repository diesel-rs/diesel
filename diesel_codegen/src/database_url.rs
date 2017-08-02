use std::borrow::Cow;
use std::env;
use std::error::Error;

pub fn extract_database_url(url: &str) -> Result<Cow<str>, String> {
    if url.starts_with("dotenv:") {
        try!(load_dotenv_file());
        extract_database_url(&url[3..])
    } else if url.starts_with("env:") {
        let var_name = &url[4..];
        env::var(var_name)
            .map(Cow::Owned)
            .map_err(|e| {
                format!("Failed to load environment variable {}: {}",
                    var_name, e.description())
            })
    } else {
        Ok(Cow::Borrowed(url))
    }
}

#[cfg(feature = "dotenv")]
fn load_dotenv_file() -> Result<(), String> {
    use dotenv::dotenv;

    dotenv().ok();
    Ok(())
}

#[cfg(not(feature = "dotenv"))]
fn load_dotenv_file() -> Result<(), String> {
    Err(String::from("diesel_codegen must be compiled with the \"dotenv\" \
        feature to correctly interpret strings starting with `dotenv:`"))
}

#[test]
fn extract_database_url_returns_the_given_string() {
    assert_eq!("foo", extract_database_url("foo").unwrap());
    assert_eq!("bar", extract_database_url("bar").unwrap());
}

#[test]
fn extract_database_url_returns_env_vars() {
    env::set_var("lolvar", "lololol");
    env::set_var("trolvar", "trolololol");
    assert_eq!("lololol", extract_database_url("env:lolvar").unwrap());
    assert_eq!("trolololol", extract_database_url("env:trolvar").unwrap());
}

#[test]
fn extract_database_url_errors_if_env_var_is_unset() {
    env::remove_var("selfdestructvar");
    assert!(extract_database_url("env:selfdestructvar").is_err());
}

use tempfile::NamedTempFile;

use std::error::Error;
use std::ffi::OsString;
use std::io::prelude::*;
use std::process::Command;

pub fn edit_string(s: &str) -> Result<String, Box<dyn Error>> {
    let mut file = NamedTempFile::new()?;
    file.write_all(s.as_bytes())?;
    editor_output(&mut file)
}

fn editor_output(file: &mut NamedTempFile) -> Result<String, Box<dyn Error>> {
    use std::io::SeekFrom::Start;

    let status = Command::new(editor_command()?)
        .arg(file.path().as_os_str())
        .spawn()?
        .wait()?;

    if !status.success() {
        return Err("Editor did not exit successfully. Aborting".into());
    }

    let mut buffer = String::new();
    file.seek(Start(0))?;
    file.read_to_string(&mut buffer)?;
    Ok(buffer)
}

fn editor_command() -> Result<OsString, Box<dyn Error>> {
    use std::env;

    env::var_os("VISUAL")
        .or_else(|| env::var_os("EDITOR"))
        .ok_or_else(|| "Either $VISUAL or $EDITOR must be set to edit files".into())
}

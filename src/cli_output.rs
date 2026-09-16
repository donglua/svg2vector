use crate::cli_error::{CliError, file_error};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

pub(crate) struct OutputFile {
    pub path: PathBuf,
    pub xml: String,
}

pub(crate) fn check_target(path: &Path, force: bool) -> Result<(), CliError> {
    let stem = path.file_stem().and_then(|stem| stem.to_str());
    let valid = stem.is_some_and(|stem| {
        let mut characters = stem.bytes();
        characters
            .next()
            .is_some_and(|first| first.is_ascii_lowercase() || first == b'_')
            && characters.all(|character| {
                character.is_ascii_lowercase() || character.is_ascii_digit() || character == b'_'
            })
    });
    if !valid || path.extension().is_none_or(|extension| extension != "xml") {
        return Err(CliError::InvalidName(path.to_owned()));
    }
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.is_dir() {
                return Err(CliError::Usage("output file path points to a directory"));
            }
            if !force {
                return Err(CliError::ExistingOutput(path.to_owned()));
            }
            Ok(())
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(file_error(path, error)),
    }
}

pub(crate) fn write_outputs(outputs: Vec<OutputFile>, force: bool) -> Result<(), CliError> {
    let mut staged = Vec::with_capacity(outputs.len());
    for output in outputs {
        let parent = match output.path.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => parent,
            Some(_) | None => Path::new("."),
        };
        fs::create_dir_all(parent).map_err(|error| file_error(parent, error))?;
        let mut temporary =
            NamedTempFile::new_in(parent).map_err(|error| file_error(parent, error))?;
        temporary
            .write_all(output.xml.as_bytes())
            .map_err(|error| file_error(&output.path, error))?;
        temporary
            .as_file()
            .sync_all()
            .map_err(|error| file_error(&output.path, error))?;
        staged.push((temporary, output.path));
    }
    for (temporary, path) in staged {
        let persisted = if force {
            temporary.persist(&path)
        } else {
            temporary.persist_noclobber(&path)
        };
        persisted.map_err(|error| file_error(&path, error.error))?;
    }
    Ok(())
}

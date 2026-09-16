use std::io;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum CliError {
    #[error("{0}")]
    Usage(&'static str),
    #[error("invalid Android resource filename '{}': use [a-z_][a-z0-9_]*.xml", .0.display())]
    InvalidName(PathBuf),
    #[error("output '{}' exists; pass --force to replace it", .0.display())]
    ExistingOutput(PathBuf),
    #[error("refusing to replace input '{}'", .0.display())]
    OverwriteInput(PathBuf),
    #[error("no direct .svg files in '{}'", .0.display())]
    EmptyDirectory(PathBuf),
    #[error("{}: {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("{}: {source}", path.display())]
    Conversion {
        path: PathBuf,
        #[source]
        source: svg2vector::Error,
    },
    #[error("standard input/output: {0}")]
    StandardIo(#[from] io::Error),
}

pub(crate) fn file_error(path: &Path, source: io::Error) -> CliError {
    CliError::Io {
        path: path.to_owned(),
        source,
    }
}

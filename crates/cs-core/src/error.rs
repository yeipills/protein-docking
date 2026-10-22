use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O error on {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("{file}:{line}: {msg}")]
    Parse {
        file: String,
        line: usize,
        msg: String,
    },
    #[error("invalid input: {0}")]
    Invalid(String),
    #[error("cancelled")]
    Cancelled,
}

impl Error {
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Error::Io {
            path: path.into(),
            source,
        }
    }

    pub fn parse(file: &str, line: usize, msg: impl Into<String>) -> Self {
        Error::Parse {
            file: file.to_string(),
            line,
            msg: msg.into(),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

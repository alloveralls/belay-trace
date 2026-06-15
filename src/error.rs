use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum BelayError {
    #[error(
        "belay-trace is not initialized at repository root {root}. Run `belay init` from this repository."
    )]
    Uninitialized { root: PathBuf },

    #[error("configuration at {path} is unavailable: {message}")]
    Config { path: PathBuf, message: String },

    #[error("configuration at {path} is invalid: {message}")]
    InvalidConfig { path: PathBuf, message: String },

    #[error("{message}")]
    Validation { message: String },

    #[error(
        "`belay {command}` is not implemented yet. This command is reserved by the v1 CLI contract."
    )]
    NotImplemented { command: &'static str },

    #[error("{message}")]
    Conflict { message: String },

    #[error("could not {action} {path}: {source}")]
    Io {
        action: &'static str,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("SQLite operation failed for {path}: {source}")]
    Sqlite {
        path: PathBuf,
        #[source]
        source: rusqlite::Error,
    },

    #[error("bundled SQLite runtime capability check failed: {message}")]
    Capability { message: String },

    #[error("{message}")]
    StorageSummary { message: String },
}

impl BelayError {
    pub const fn exit_code(&self) -> u8 {
        match self {
            Self::Uninitialized { .. } | Self::Config { .. } => 3,
            Self::InvalidConfig { .. } | Self::Validation { .. } | Self::NotImplemented { .. } => 4,
            Self::Conflict { .. } => 5,
            Self::Io { .. }
            | Self::Sqlite { .. }
            | Self::Capability { .. }
            | Self::StorageSummary { .. } => 6,
        }
    }

    pub fn io(action: &'static str, path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            action,
            path: path.into(),
            source,
        }
    }

    pub fn sqlite(path: impl Into<PathBuf>, source: rusqlite::Error) -> Self {
        Self::Sqlite {
            path: path.into(),
            source,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CaveError {
    #[error("No cave is currently open")]
    NoCaveOpen,

    #[error("Note not found: {0}")]
    NotFound(String),

    #[error("Note already exists: {0}")]
    AlreadyExists(String),

    #[error("Invalid note name: {0}")]
    InvalidName(String),

    #[error("Text to replace not found in note")]
    EditNotFound,

    #[error("IO error: {0}")]
    Io(String),

    #[error("Internal state error: mutex poisoned")]
    Poisoned,
}

impl From<std::io::Error> for CaveError {
    fn from(e: std::io::Error) -> Self {
        CaveError::Io(e.to_string())
    }
}

impl serde::Serialize for CaveError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

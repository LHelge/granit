#[derive(Debug, thiserror::Error)]
pub enum CaveError {
    #[error("No cave is currently open")]
    NoCaveOpen,

    #[error("Note not found: {0}")]
    NotFound(String),

    #[error("Template not found: {0}")]
    TemplateNotFound(String),

    #[error("Note already exists: {0}")]
    AlreadyExists(String),

    #[error("Template already exists: {0}")]
    TemplateAlreadyExists(String),

    #[error("Duplicate slug \"{slug}\" found at {paths:?}")]
    DuplicateSlug { slug: String, paths: Vec<String> },

    #[error("Duplicate template slug \"{slug}\" found at {paths:?}")]
    DuplicateTemplateSlug { slug: String, paths: Vec<String> },

    #[error("Invalid note name: {0}")]
    InvalidName(String),

    #[error("Exhausted slug range for base name: {0}")]
    SlugExhausted(String),

    #[error("Text to replace not found in note")]
    EditNotFound,

    #[error("Line {0} is not a todo checkbox")]
    InvalidTodoLine(usize),

    #[error("IO error: {0}")]
    Io(String),

    #[error("YAML parse error: {0}")]
    Yaml(String),

    #[error("Template render error: {0}")]
    TemplateRender(String),
}

impl From<std::io::Error> for CaveError {
    fn from(e: std::io::Error) -> Self {
        CaveError::Io(e.to_string())
    }
}

impl From<serde_yml::Error> for CaveError {
    fn from(e: serde_yml::Error) -> Self {
        CaveError::Yaml(e.to_string())
    }
}

impl From<tera::Error> for CaveError {
    fn from(e: tera::Error) -> Self {
        CaveError::TemplateRender(e.to_string())
    }
}

impl CaveError {
    /// Stable, machine-readable code sent over IPC. Must not change casually
    /// — the frontend branches on these strings.
    pub fn code(&self) -> &'static str {
        match self {
            CaveError::NoCaveOpen => "NoCaveOpen",
            CaveError::NotFound(_) => "NotFound",
            CaveError::TemplateNotFound(_) => "TemplateNotFound",
            CaveError::AlreadyExists(_) => "AlreadyExists",
            CaveError::TemplateAlreadyExists(_) => "TemplateAlreadyExists",
            CaveError::DuplicateSlug { .. } => "DuplicateSlug",
            CaveError::DuplicateTemplateSlug { .. } => "DuplicateTemplateSlug",
            CaveError::InvalidName(_) => "InvalidName",
            CaveError::SlugExhausted(_) => "SlugExhausted",
            CaveError::EditNotFound => "EditNotFound",
            CaveError::InvalidTodoLine(_) => "InvalidTodoLine",
            CaveError::Io(_) => "Io",
            CaveError::Yaml(_) => "Yaml",
            CaveError::TemplateRender(_) => "TemplateRender",
        }
    }
}

impl From<CaveError> for granit_types::IpcError {
    fn from(e: CaveError) -> Self {
        granit_types::IpcError::new(e.code(), e.to_string())
    }
}

impl From<&CaveError> for granit_types::IpcError {
    fn from(e: &CaveError) -> Self {
        granit_types::IpcError::new(e.code(), e.to_string())
    }
}

impl serde::Serialize for CaveError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        granit_types::IpcError::from(self).serialize(serializer)
    }
}

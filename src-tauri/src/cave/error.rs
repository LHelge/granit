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

    #[error("Duplicate link target \"{slug}\": heading anchor in \"{anchor_note}\" collides with {conflict}")]
    DuplicateAnchor {
        slug: String,
        anchor_note: String,
        conflict: String,
    },

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

impl serde::Serialize for CaveError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

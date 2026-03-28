#[derive(Debug, thiserror::Error, serde::Serialize)]
pub enum AgentError {
    #[error("Failed to build agent: {0}")]
    Build(String),
}

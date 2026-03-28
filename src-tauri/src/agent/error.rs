#[derive(Debug, thiserror::Error, serde::Serialize)]
pub enum AgentError {
    #[error("Failed to build agent: {0}")]
    Build(String),
    #[error("Streaming error: {0}")]
    Stream(String),
    #[error("Agent not initialized — open a cave first")]
    NotInitialized,
    #[error("State lock poisoned")]
    Poisoned,
}

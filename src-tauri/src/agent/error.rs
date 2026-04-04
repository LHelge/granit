#[derive(Debug, thiserror::Error, serde::Serialize)]
pub enum AgentError {
    #[error("Failed to build agent: {0}")]
    Build(String),
    #[error("Streaming error: {0}")]
    Stream(String),
    #[error("Agent not initialized — open a cave first")]
    NotInitialized,
    #[error("No providers configured")]
    NoProviders,
    #[error("Selected provider index {0} is out of range")]
    ProviderIndexOutOfRange(usize),
    #[error("Failed to list models: {0}")]
    ModelListing(String),
}

impl From<rig::http_client::Error> for AgentError {
    fn from(e: rig::http_client::Error) -> Self {
        Self::Build(e.to_string())
    }
}

impl From<rig::model::ModelListingError> for AgentError {
    fn from(e: rig::model::ModelListingError) -> Self {
        Self::ModelListing(e.to_string())
    }
}

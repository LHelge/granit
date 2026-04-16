#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("Failed to build agent: {0}")]
    Build(String),
    #[error("Streaming error: {0}")]
    Stream(String),
    #[error("Streaming timed out: no response for {0} seconds")]
    StreamTimeout(u64),
    #[error("Agent not initialized — open a cave first")]
    NotInitialized,
    #[error("No providers configured")]
    NoProviders,
    #[error("Selected provider index {0} is out of range")]
    ProviderIndexOutOfRange(usize),
    #[error("Failed to list models: {0}")]
    ModelListing(String),
}

impl AgentError {
    /// Stable, machine-readable code sent over IPC.
    pub fn code(&self) -> &'static str {
        match self {
            AgentError::Build(_) => "AgentBuild",
            AgentError::Stream(_) => "AgentStream",
            AgentError::StreamTimeout(_) => "AgentStreamTimeout",
            AgentError::NotInitialized => "AgentNotInitialized",
            AgentError::NoProviders => "AgentNoProviders",
            AgentError::ProviderIndexOutOfRange(_) => "AgentProviderIndexOutOfRange",
            AgentError::ModelListing(_) => "AgentModelListing",
        }
    }
}

impl From<AgentError> for granit_types::IpcError {
    fn from(e: AgentError) -> Self {
        granit_types::IpcError::new(e.code(), e.to_string())
    }
}

impl From<&AgentError> for granit_types::IpcError {
    fn from(e: &AgentError) -> Self {
        granit_types::IpcError::new(e.code(), e.to_string())
    }
}

impl serde::Serialize for AgentError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        granit_types::IpcError::from(self).serialize(serializer)
    }
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

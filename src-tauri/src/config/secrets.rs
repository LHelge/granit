use std::collections::HashMap;

/// Typed access to secrets loaded from `secrets.env` files.
#[derive(Debug, Clone)]
pub struct Secrets {
    vars: HashMap<String, String>,
}

impl Secrets {
    pub fn new(vars: HashMap<String, String>) -> Self {
        Self { vars }
    }

    /// Get the agent API key (`AGENT_API_KEY`).
    pub fn agent_api_key(&self) -> Option<&str> {
        self.vars.get("AGENT_API_KEY").map(|s| s.as_str())
    }

    /// Get an arbitrary secret by key name.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.vars.get(key).map(|s| s.as_str())
    }
}

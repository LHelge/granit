use serde::{Deserialize, Serialize};

/// Typed error envelope serialized over the Tauri IPC boundary.
///
/// Backend command results (`CaveError`, `AgentError`, `ConfigError`, …)
/// all serialize into this shape so the frontend can branch on `code`
/// without string-matching on `message`.
///
/// `code` is a stable, machine-readable identifier such as `"NotFound"` or
/// `"AlreadyExists"`. `message` is a human-readable description suitable
/// for display or logging.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IpcError {
    pub code: String,
    pub message: String,
}

impl IpcError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }

    /// Convenience constructor for errors that have no structured code.
    pub fn other(message: impl Into<String>) -> Self {
        Self::new("Other", message)
    }

    /// Returns true if the error's `code` matches `expected`.
    pub fn is(&self, expected: &str) -> bool {
        self.code == expected
    }
}

impl std::fmt::Display for IpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Keep the display human-readable; callers log or render this.
        f.write_str(&self.message)
    }
}

impl std::error::Error for IpcError {}

impl From<IpcError> for String {
    /// Stringifies to the human-readable `message`. Loses the `code`;
    /// use [`IpcError::is`] or pattern-match on the struct where the
    /// structure matters.
    fn from(e: IpcError) -> Self {
        e.message
    }
}

use serde::{Deserialize, Serialize};

/// Release notes for the version the app is currently running, shown once
/// after an automatic update has been applied.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseNotes {
    pub version: String,
    /// Notes rendered to HTML by the backend markdown pipeline.
    pub notes_html: String,
}

/// Outcome of an update check.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum UpdateCheckStatus {
    /// No newer version was available.
    UpToDate,
    /// A newer version was downloaded and installed; it applies on the
    /// next launch (or via an explicit restart).
    Installed { version: String },
}

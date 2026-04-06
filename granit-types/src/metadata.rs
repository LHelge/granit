use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppMetadata {
    pub app_name: String,
    pub repo_url: String,
    pub version: String,
    pub git_commit_hash: String,
    pub git_dirty: bool,
}

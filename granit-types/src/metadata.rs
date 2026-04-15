use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppMetadata {
    pub app_name: String,
    pub repo_url: String,
    pub version: String,
    pub git_commit_hash: String,
    pub git_dirty: bool,
}

impl AppMetadata {
    const APP_NAME: &'static str = "Granit";
    const REPO_URL: &'static str = "https://github.com/LHelge/granit";

    pub fn new(git_commit_hash: &str, git_dirty: bool) -> Self {
        let git_commit_hash = git_commit_hash.chars().take(8).collect();

        AppMetadata {
            app_name: Self::APP_NAME.to_string(),
            repo_url: Self::REPO_URL.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            git_commit_hash,
            git_dirty,
        }
    }
}

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

    pub fn from_env() -> Self {
        let git_commit_hash = option_env!("GRANIT_GIT_HASH")
            .unwrap_or("unknown")
            .chars()
            .take(8)
            .collect();

        AppMetadata {
            app_name: Self::APP_NAME.to_string(),
            repo_url: Self::REPO_URL.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            git_commit_hash,
            git_dirty: option_env!("GRANIT_GIT_DIRTY").unwrap_or("false") == "true",
        }
    }
}

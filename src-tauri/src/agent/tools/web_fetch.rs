use html_to_markdown_rs::{convert, ConversionOptions, PreprocessingPreset};
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::Deserialize;
use serde_json::json;
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(10);
const MAX_BODY_SIZE: usize = 512 * 1024; // 512 KB
const MAX_OUTPUT_CHARS: usize = 100_000;

#[derive(Clone)]
pub struct WebFetchTool {
    client: reqwest::Client,
}

impl WebFetchTool {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent("Granit/1.0")
            .timeout(TIMEOUT)
            .build()
            .expect("failed to create HTTP client");

        Self { client }
    }
}

#[derive(Deserialize)]
pub struct WebFetchArgs {
    url: String,
}

#[derive(Debug, thiserror::Error)]
pub enum WebFetchError {
    #[error("HTTP request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("HTML to markdown conversion failed: {0}")]
    Conversion(String),
    #[error("{0}")]
    Other(String),
}

impl Tool for WebFetchTool {
    const NAME: &'static str = "web_fetch";
    type Error = WebFetchError;
    type Args = WebFetchArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Fetch a webpage and return its content as markdown. Use this to read \
                          the full content of a specific URL found via web search."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "The URL of the webpage to fetch"
                    }
                },
                "required": ["url"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let response = self.client.get(&args.url).send().await?;

        if !response.status().is_success() {
            return Err(WebFetchError::Other(format!(
                "Page returned status {}",
                response.status()
            )));
        }

        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if !content_type.contains("text/html") {
            return Err(WebFetchError::Other(format!(
                "Expected text/html, got: {content_type}"
            )));
        }

        let bytes = response.bytes().await?;

        if bytes.len() > MAX_BODY_SIZE {
            return Err(WebFetchError::Other(format!(
                "Response body too large ({} bytes, max {MAX_BODY_SIZE})",
                bytes.len()
            )));
        }

        let html = String::from_utf8_lossy(&bytes);

        let mut options = ConversionOptions::default();
        options.preprocessing.preset = PreprocessingPreset::Aggressive;
        options.preprocessing.remove_navigation = true;
        options.preprocessing.remove_forms = true;
        options.skip_images = true;

        let result =
            convert(&html, Some(options)).map_err(|e| WebFetchError::Conversion(e.to_string()))?;

        let markdown = result.content.unwrap_or_default();

        if markdown.len() > MAX_OUTPUT_CHARS {
            Ok(markdown[..MAX_OUTPUT_CHARS].to_string())
        } else {
            Ok(markdown)
        }
    }
}

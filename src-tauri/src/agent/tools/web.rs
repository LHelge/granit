use granit_types::{WebFetchConfig, WebSearchConfig};
use html_to_markdown_rs::{convert, ConversionOptions, PreprocessingPreset};
use rig_core::completion::ToolDefinition;
use rig_core::tool::Tool;
use serde::Deserialize;
use serde_json::json;
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(10);
const MAX_BODY_SIZE: usize = 512 * 1024; // 512 KB

// ── web_fetch ──────────────────────────────────────────────────────

#[derive(Clone)]
pub struct WebFetchTool {
    client: reqwest::Client,
    max_output_chars: usize,
}

impl WebFetchTool {
    pub fn new(config: &WebFetchConfig) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("Granit/1.0")
            .timeout(TIMEOUT)
            .build()
            .expect("failed to create HTTP client");

        Self {
            client,
            max_output_chars: config.max_output_chars,
        }
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
        options.strip_tags = vec![
            "svg".to_string(),
            "iframe".to_string(),
            "video".to_string(),
            "audio".to_string(),
            "canvas".to_string(),
            "noscript".to_string(),
        ];

        let result =
            convert(&html, Some(options)).map_err(|e| WebFetchError::Conversion(e.to_string()))?;

        let markdown = result.content.unwrap_or_default();

        if markdown.len() > self.max_output_chars {
            Ok(markdown[..self.max_output_chars].to_string())
        } else {
            Ok(markdown)
        }
    }
}

// ── web_search ─────────────────────────────────────────────────────

#[derive(Clone)]
pub struct WebSearchTool {
    client: reqwest::Client,
    api_key: String,
    max_results: usize,
}

#[derive(Deserialize, Debug)]
struct BraveResponse {
    web: Option<BraveWebResults>,
}

#[derive(Deserialize, Debug)]
struct BraveWebResults {
    results: Vec<BraveWebResult>,
}

#[derive(Deserialize, Debug)]
struct BraveWebResult {
    title: String,
    url: String,
    description: String,
}

impl WebSearchTool {
    pub fn new(config: &WebSearchConfig) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("Granit/1.0")
            .build()
            .expect("failed to create HTTP client");

        Self {
            client,
            api_key: config.api_key.clone().unwrap_or_default(),
            max_results: config.max_results,
        }
    }
}

#[derive(Deserialize)]
pub struct WebSearchArgs {
    query: String,
}

#[derive(Debug, thiserror::Error)]
pub enum WebSearchError {
    #[error("HTTP request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("{0}")]
    Other(String),
}

impl Tool for WebSearchTool {
    const NAME: &'static str = "web_search";
    type Error = WebSearchError;
    type Args = WebSearchArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Search the web using Brave Search. Returns a list of relevant results \
                          with titles, URLs, and descriptions."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The search query to look up on the web"
                    }
                },
                "required": ["query"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let response = self
            .client
            .get("https://api.search.brave.com/res/v1/web/search")
            .header("X-Subscription-Token", &self.api_key)
            .query(&[("q", &args.query), ("count", &self.max_results.to_string())])
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(WebSearchError::Other(format!(
                "Brave search returned status {}",
                response.status()
            )));
        }

        let brave: BraveResponse = response.json().await?;

        let results = brave.web.map(|w| w.results).unwrap_or_default();

        if results.is_empty() {
            return Ok("No results found.".to_string());
        }

        let formatted = results
            .iter()
            .enumerate()
            .map(|(i, r)| format!("{}. {}\n   {}\n   {}", i + 1, r.title, r.url, r.description))
            .collect::<Vec<_>>()
            .join("\n\n");

        Ok(formatted)
    }
}

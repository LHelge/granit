use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::Deserialize;
use serde_json::json;

#[derive(Clone)]
pub struct WebSearchTool {
    client: reqwest::Client,
    api_key: String,
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
    pub fn new(api_key: &str) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("Granit/1.0")
            .build()
            .expect("failed to create HTTP client");

        Self {
            client,
            api_key: api_key.to_string(),
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
            .query(&[("q", &args.query), ("count", &"5".to_string())])
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

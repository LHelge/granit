use rig_core::tool::PortableTool;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::agent::vectordb::CaveVectorIndex;
use crate::agent::AgentError;

// ── semantic_search ────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct SemanticSearchArgs {
    /// Natural-language description of what to find.
    query: String,
    /// Maximum number of results (defaults to the configured RAG top_n).
    max_results: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct SemanticMatch {
    slug: String,
    /// Cosine similarity in [-1, 1]; higher is more similar.
    score: f64,
}

#[derive(Debug, Serialize)]
pub struct SemanticSearchOutput {
    results: Vec<SemanticMatch>,
}

pub struct SemanticSearchTool {
    pub index: CaveVectorIndex,
    pub default_top_n: usize,
}

impl PortableTool for SemanticSearchTool {
    const NAME: &'static str = "semantic_search";
    type Error = AgentError;
    type Args = SemanticSearchArgs;
    type Output = SemanticSearchOutput;

    fn description(&self) -> String {
        "Find notes semantically related to a query using vector embeddings, ranked by similarity. Use this to locate notes by meaning when keyword search (search_notes/search_content) is too literal; read promising results with read_note. Results may be incomplete while the cave is still being indexed."
            .to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Natural-language description of the content to find."
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum number of results to return."
                }
            },
            "required": ["query"]
        })
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let n = args.max_results.unwrap_or(self.default_top_n).max(1);
        let results = self
            .index
            .search(&args.query, n, None)
            .await?
            .into_iter()
            .map(|(score, slug)| SemanticMatch { slug, score })
            .collect();
        Ok(SemanticSearchOutput { results })
    }
}

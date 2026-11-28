//! The conversation as one Markdown document.
//!
//! The whole chat is a single document in one `TextView`, so a selection
//! can run across messages and blocks, and the view follows the bottom as
//! one list. Each turn is appended: the user's message, tool calls and
//! errors as top-level blockquotes opened by a `[!user]`, `[!tool]` or
//! `[!error]` marker (drawn as GPUI Kit block surfaces, see
//! [`super::SURFACES`]), the assistant's reply as plain Markdown between
//! them. Every method returns the text it appended, for
//! `TextViewState::push_str`; the document only ever grows until it is
//! cleared.

use granit_types::ToolCallInfo;

use super::wiki_links::WikiLinkRewriter;

/// The surface name of a user message.
pub const USER_SURFACE: &str = "user";
/// The surface name of a tool call.
pub const TOOL_SURFACE: &str = "tool";
/// The surface name of an error.
pub const ERROR_SURFACE: &str = "error";

#[derive(Default)]
pub struct Transcript {
    source: String,
    /// Rewrites the wiki-links of the reply being streamed.
    reply: WikiLinkRewriter,
    /// Link targets appended since the last [`Self::take_targets`].
    targets: Vec<String>,
}

impl Transcript {
    /// The whole document.
    #[cfg(test)]
    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn is_empty(&self) -> bool {
        self.source.is_empty()
    }

    /// Start over with an empty document.
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// The wiki-link targets appended since the last call, to resolve.
    pub fn take_targets(&mut self) -> Vec<String> {
        let mut targets = std::mem::take(&mut self.targets);
        targets.extend(self.reply.take_targets());
        targets
    }

    /// Append the user's message, as Markdown with its wiki-links.
    pub fn push_user(&mut self, text: &str) -> String {
        let (markdown, targets) = WikiLinkRewriter::rewrite(text.trim_end());
        self.targets.extend(targets);
        self.push_block(USER_SURFACE, &markdown)
    }

    /// Append a chunk of the streamed reply.
    pub fn push_reply(&mut self, chunk: &str) -> String {
        let text = self.reply.feed(chunk);
        self.append(text)
    }

    /// The reply ended (or was interrupted): flush what the wiki-link
    /// rewriter still holds.
    pub fn finish_reply(&mut self) -> String {
        let text = self.reply.finish();
        self.append(text)
    }

    /// Append a tool call, `name(param)`. Ends the reply text before it.
    pub fn push_tool_call(&mut self, info: &ToolCallInfo) -> String {
        let call = format!(
            "{}({})",
            info.name,
            info.param.as_deref().unwrap_or_default()
        );
        let mut text = self.finish_reply();
        text.push_str(&self.push_block(TOOL_SURFACE, &inline_code(&call)));
        text
    }

    /// Append an error message, as plain text. Ends the reply text before
    /// it.
    pub fn push_error(&mut self, message: &str) -> String {
        let mut text = self.finish_reply();
        text.push_str(&self.push_block(ERROR_SURFACE, &escape_markdown(message)));
        text
    }

    /// Append `markdown` as a surface block, on its own after a blank line.
    fn push_block(&mut self, surface: &str, markdown: &str) -> String {
        let mut text = self.separator();
        text.push_str(&format!("> [!{surface}]\n"));
        for line in markdown.lines() {
            if line.is_empty() {
                text.push_str(">\n");
            } else {
                text.push_str("> ");
                text.push_str(line);
                text.push('\n');
            }
        }
        // The blank line ends the blockquote, so the text after it is not
        // a lazy continuation.
        text.push('\n');
        self.append(text)
    }

    /// What ends the document's last block, so the next one starts fresh.
    fn separator(&self) -> String {
        if self.source.is_empty() || self.source.ends_with("\n\n") {
            String::new()
        } else if self.source.ends_with('\n') {
            "\n".to_string()
        } else {
            "\n\n".to_string()
        }
    }

    fn append(&mut self, text: String) -> String {
        self.source.push_str(&text);
        text
    }
}

/// `text` as an inline code span, whatever backticks it contains, on one
/// line.
fn inline_code(text: &str) -> String {
    let text = text.replace(['\n', '\r'], " ");
    let longest = text.split(|c| c != '`').map(str::len).max().unwrap_or(0);
    let fence = "`".repeat(longest + 1);
    let pad = if text.starts_with('`') || text.ends_with('`') {
        " "
    } else {
        ""
    };
    format!("{fence}{pad}{text}{pad}{fence}")
}

/// `text` with the characters Markdown could read as markup escaped.
fn escape_markdown(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if c.is_ascii_punctuation() {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turns_become_surface_blocks_and_reply_text() {
        let mut transcript = Transcript::default();
        let mut pushed = transcript.push_user("Hi [[a]]\n\nsecond");
        pushed.push_str(&transcript.push_reply("Reading "));
        pushed.push_str(&transcript.push_tool_call(&ToolCallInfo {
            name: "read_note".into(),
            param: Some("a".into()),
        }));
        pushed.push_str(&transcript.push_reply("Done, see [["));
        pushed.push_str(&transcript.push_reply("b]]"));
        pushed.push_str(&transcript.finish_reply());
        pushed.push_str(&transcript.push_error("Boom *bold*"));

        let expected = concat!(
            "> [!user]\n> Hi [a](<wiki:a>)\n>\n> second\n\n",
            "Reading \n\n> [!tool]\n> `read_note(a)`\n\n",
            "Done, see [b](<wiki:b>)\n\n> [!error]\n> Boom \\*bold\\*\n\n",
        );
        assert_eq!(transcript.source(), expected);
        assert_eq!(
            pushed,
            transcript.source(),
            "the pushed text is the document"
        );
        assert_eq!(transcript.take_targets(), vec!["a", "b"]);
    }

    #[test]
    fn inline_code_survives_backticks() {
        assert_eq!(inline_code("a"), "`a`");
        assert_eq!(inline_code("a`b"), "``a`b``");
        assert_eq!(inline_code("`x`"), "`` `x` ``");
    }
}

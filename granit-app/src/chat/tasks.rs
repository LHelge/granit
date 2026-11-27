//! `/task` invocations typed into the chat input.
//!
//! Tasks are the prompts in `.granit/agent/tasks/`. Typing `/` at the
//! start of the input lists the tasks whose names start with what follows;
//! `/name rest of text` sends the task `name` with `rest of text` as its
//! input. Ported from the Tauri app's chat panel.

use granit_types::AgentDocInfo;

/// Parse a task invocation out of a typed message.
///
/// `/name rest of text` (or exactly `/name`) invokes the task `name` with
/// `rest of text` as its input. Task names may contain spaces, so the
/// longest matching name wins. Returns `None` when the message is not a
/// task invocation: it is then sent as a plain message.
pub fn parse_task_invocation(raw: &str, tasks: &[AgentDocInfo]) -> Option<(String, String)> {
    let rest = raw.strip_prefix('/')?;
    let name = tasks
        .iter()
        .map(|t| t.name.as_str())
        .filter(|name| {
            rest == *name
                || rest
                    .strip_prefix(name)
                    .is_some_and(|r| r.starts_with(char::is_whitespace))
        })
        .max_by_key(|name| name.len())?;
    let input = rest[name.len()..].trim().to_string();
    Some((name.to_string(), input))
}

/// The tasks to offer while `input` is being typed: those whose names
/// start with the text after a leading `/`, case-insensitively. None when
/// the input does not start with `/`.
pub fn matching_tasks<'a>(input: &str, tasks: &'a [AgentDocInfo]) -> Vec<&'a AgentDocInfo> {
    let Some(query) = input.strip_prefix('/') else {
        return Vec::new();
    };
    let query = query.to_lowercase();
    tasks
        .iter()
        .filter(|task| task.name.to_lowercase().starts_with(&query))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tasks(names: &[&str]) -> Vec<AgentDocInfo> {
        names
            .iter()
            .map(|n| AgentDocInfo {
                name: n.to_string(),
                description: String::new(),
            })
            .collect()
    }

    #[test]
    fn plain_messages_are_not_task_invocations() {
        let tasks = tasks(&["summarize"]);
        assert_eq!(parse_task_invocation("hello world", &tasks), None);
        assert_eq!(parse_task_invocation("/unknown-task text", &tasks), None);
        assert_eq!(parse_task_invocation("/summarizer", &tasks), None);
    }

    #[test]
    fn task_invocation_with_and_without_input() {
        let tasks = tasks(&["summarize"]);
        assert_eq!(
            parse_task_invocation("/summarize", &tasks),
            Some(("summarize".to_string(), String::new()))
        );
        assert_eq!(
            parse_task_invocation("/summarize chapter two", &tasks),
            Some(("summarize".to_string(), "chapter two".to_string()))
        );
    }

    #[test]
    fn longest_task_name_wins_for_names_with_spaces() {
        let tasks = tasks(&["daily", "daily review"]);
        assert_eq!(
            parse_task_invocation("/daily review extra input", &tasks),
            Some(("daily review".to_string(), "extra input".to_string()))
        );
        assert_eq!(
            parse_task_invocation("/daily stand-up", &tasks),
            Some(("daily".to_string(), "stand-up".to_string()))
        );
    }

    #[test]
    fn matching_tasks_filters_by_prefix() {
        let tasks = tasks(&["Daily", "weekly"]);
        let names = |input| -> Vec<String> {
            matching_tasks(input, &tasks)
                .into_iter()
                .map(|t| t.name.clone())
                .collect()
        };
        assert_eq!(names("/d"), vec!["Daily"]);
        assert_eq!(names("/"), vec!["Daily", "weekly"]);
        assert!(names("d").is_empty());
    }
}

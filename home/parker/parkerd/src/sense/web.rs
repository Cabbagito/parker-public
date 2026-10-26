// Web search, through the machine's own SearXNG (system/searxng.nix): a
// metasearch engine on localhost, so parker's queries cost nothing, need no
// key, and transit no provider that can change its terms. Shelled out to
// curl in house style — a one-shot read does not care about a fork, and
// `--data-urlencode` is a URL encoder we do not have to write.

use serde_json::Value;
use std::process::Command;

const ENDPOINT: &str = "http://127.0.0.1:8888/search";

/// How many results to hand a model. Snippets carry most of the value; a
/// long tail is tokens spent on rank 15.
const RESULTS: usize = 8;

pub fn search(query: &str) -> Result<String, String> {
    let query = query.trim();
    if query.is_empty() {
        return Err("search needs a query".into());
    }

    let out = Command::new("curl")
        .args([
            "-sf",
            "--max-time",
            "10",
            "--get",
            ENDPOINT,
            "--data-urlencode",
            &format!("q={query}"),
            "--data-urlencode",
            "format=json",
        ])
        .output()
        .map_err(|e| format!("could not run curl: {e}"))?;

    if !out.status.success() {
        return Err("search is unreachable — is searxng.service running?".into());
    }

    let v: Value = serde_json::from_slice(&out.stdout)
        .map_err(|e| format!("searxng answered something unreadable: {e}"))?;
    Ok(render(&v, query))
}

/// Text, not JSON, for the same reason sense renders text: the reader is a
/// model and the JSON of the same content costs ~3× the tokens.
fn render(v: &Value, query: &str) -> String {
    let mut lines = Vec::new();

    // Instant answers (unit conversions, definitions) come ahead of results
    // when an engine produces one — often the whole errand.
    for a in v
        .get("answers")
        .and_then(Value::as_array)
        .unwrap_or(&Vec::new())
    {
        // Answers have been both bare strings and {answer: ...} objects
        // across SearXNG versions; take either, assume neither.
        if let Some(text) = a
            .as_str()
            .or_else(|| a.get("answer").and_then(Value::as_str))
        {
            lines.push(format!("answer: {text}"));
        }
    }

    let empty = Vec::new();
    let results = v.get("results").and_then(Value::as_array).unwrap_or(&empty);
    if results.is_empty() && lines.is_empty() {
        return format!("nothing found for '{query}'");
    }

    for (i, r) in results.iter().take(RESULTS).enumerate() {
        let title = r
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("(untitled)");
        let url = r.get("url").and_then(Value::as_str).unwrap_or("");
        lines.push(format!("{}. {title}\n   {url}", i + 1));
        if let Some(snippet) = r.get("content").and_then(Value::as_str) {
            let snippet = snippet.trim();
            if !snippet.is_empty() {
                lines.push(format!("   {snippet}"));
            }
        }
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn an_empty_query_is_refused_before_any_network() {
        assert!(search("   ").is_err());
    }

    #[test]
    fn results_render_ranked_with_url_and_snippet() {
        let v = json!({ "results": [
            { "title": "NixOS", "url": "https://nixos.org", "content": "The distro." },
            { "title": "Bare", "url": "https://b.example" },
        ]});
        let text = render(&v, "nixos");
        assert!(text.contains("1. NixOS\n   https://nixos.org\n   The distro."));
        assert!(text.contains("2. Bare"));
    }

    #[test]
    fn answers_come_first_in_either_shape() {
        let v = json!({
            "answers": ["42", { "answer": "forty-two" }],
            "results": [{ "title": "t", "url": "u" }]
        });
        let text = render(&v, "q");
        assert!(text.starts_with("answer: 42\nanswer: forty-two\n"));
    }

    #[test]
    fn nothing_found_says_so_instead_of_returning_blank() {
        assert_eq!(render(&json!({}), "xyzzy"), "nothing found for 'xyzzy'");
    }

    #[test]
    fn the_tail_beyond_the_cap_is_dropped() {
        let many: Vec<_> = (0..20)
            .map(|i| json!({ "title": format!("r{i}"), "url": "u" }))
            .collect();
        let text = render(&json!({ "results": many }), "q");
        assert!(text.contains("8. r7"));
        assert!(!text.contains("9. r8"));
    }
}

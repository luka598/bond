use std::time::Duration;
use crate::{agent::function::Functions, bond::functions::manual::Manual, taglang::Tag};

pub const ENABLE_WEB_SEARCH: bool = true;

pub const MANUAL: &'static str = r#"
# Web Module Manual

This module provides controlled web interaction utilities: fetching arbitrary URLs and performing simple web searches. These functions are meant to be used by the LLM runtime and must return structured JSON-like strings to indicate success, output, and error information.

## Configuration

- ENABLE_WEB_SEARCH: a boolean constant that gates whether web search is allowed. Set to false to disable web search.

## Functions

- web::fetch(url: string, prompt: string) -> String
   - Fetches the provided URL (HTTP GET) with a 10 second timeout.
   - If the response Content-Type contains "text/html" the body will be converted to plain text using html2text. Non-HTML responses are returned as raw text.
   - Returns a JSON string: {"success": <bool>, "output": <string>, "error": <string>}.
   - On HTTP errors (status >= 400) or network errors returns success=false with an error message.

- web::search(query: string) -> String
   - Performs a POST search against DuckDuckGo's HTML endpoint (https://html.duckduckgo.com/html) with a 10 second timeout.
   - Requires ENABLE_WEB_SEARCH to be true, otherwise returns success=false and an explanatory error.
   - The HTML response is converted to plain text via html2text and then sliced (emulates dropping the first 713 characters of the markdown) to remove boilerplate.
   - Returns the same JSON structure as web_fetch.

## Helper utilities

- url_encode(s: string) -> string
   - Encodes query values for application/x-www-form-urlencoded use. Spaces become '+', unsafe bytes are percent-encoded.

- format_json(success: bool, output: string, error: string) -> string
   - Helper to produce the JSON string used by both functions. Strings are JSON-quoted using serde_json to ensure proper escaping.

## Behavior notes and safety

- Timeouts: all network calls use a 10 second timeout to avoid blocking the runtime.
- HTML handling: html2text is used to convert HTML to readable text. If conversion fails an error is returned.
- Search gating: the ENABLE_WEB_SEARCH flag exists to prevent accidental use of the search function in restricted environments.
- Output size: callers should be prepared to handle potentially large output. The search implementation trims an initial chunk of the converted markdown (713 characters) to remove static header text; this is an implementation detail and may change.

## Example calls

- fetch:
  - args: {"url": "https://example.com", "prompt": "optional prompt"}
  - returns: format_json(true, "...converted text...", "") on success

- search:
  - args: {"query": "rust async http client"}
  - returns: format_json(true, "...search result text...", "") on success


"#;

pub fn register(f: &mut Functions, m: &mut Manual) {
    m.register("web", MANUAL);
    f.register("web::fetch", web_fetch);
    f.register("web::search", web_search);
}

fn url_encode(s: &str) -> String {
    let mut out = String::new();
    for &b in s.as_bytes() {
        match b {
            b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

async fn web_fetch(x: Tag) -> String {
    let url = x.get("url").unwrap().as_str();
    let client = reqwest::Client::new();

    let resp = match client.get(url).timeout(Duration::from_secs(10)).send().await {
        Ok(r) => r,
        Err(e) => return format!("error fetching {}: {}", url, e),
    };

    if resp.status().as_u16() >= 400 {
        return format!("error fetching {}: HTTP {}", url, resp.status().as_u16());
    }

    let content_type = resp.headers()
        .get("Content-Type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    if content_type.contains("text/html") {
        match resp.text().await {
            Ok(text) => match html2text::from_read(text.as_bytes(), 80) {
                Ok(md) => md,
                Err(e) => format!("error converting HTML {}: {}", url, e),
            },
            Err(e) => format!("error fetching {}: {}", url, e),
        }
    } else {
        match resp.text().await {
            Ok(text) => text,
            Err(e) => format!("error fetching {}: {}", url, e),
        }
    }
}

async fn web_search(x: Tag) -> String {
    if !ENABLE_WEB_SEARCH {
        return "web search is disabled".to_string();
    }

    let query = x.get("query").unwrap().as_str();
    let client = reqwest::Client::new();
    let form_body = format!("q={}", url_encode(query));

    let resp = match client
        .post("https://html.duckduckgo.com/html")
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(form_body)
        .timeout(Duration::from_secs(10))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return format!("error searching for {}: {}", query, e),
    };

    if resp.status().as_u16() >= 400 {
        return format!("error searching for {}: HTTP {}", query, resp.status().as_u16());
    }

    let response_text = match resp.text().await {
        Ok(t) => t,
        Err(e) => return format!("error searching for {}: {}", query, e),
    };

    match html2text::from_read(response_text.as_bytes(), 80) {
        Ok(md) => {
            if md.chars().count() > 713 {
                md.chars().skip(713).collect()
            } else {
                String::new()
            }
        }
        Err(e) => format!("error converting search HTML for {}: {}", query, e),
    }
}
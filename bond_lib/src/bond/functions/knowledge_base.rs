use tokio::fs;
use std::path::Path;
use crate::{agent::function::Functions, bond::functions::manual::Manual, taglang::Tag};

pub const MANUAL: &'static str = r#"
# Knowledge Base Module Manual

This is the ONLY authorized tool for searching the local knowledge base.
The knowledge base lives in the `kb/` directory and may contain arbitrary subdirectories. Only `.md` files are searched.

## Functions

- knowledge_base::search(q1: string, q2: string, q3: string, top_n: string) -> String
   - Searches the knowledge base using up to three independent queries.
   - Pass "" for any query you want to skip. At least one must be non-empty.
   - Queries do NOT need to be related — they work together additively. A document matching multiple queries ranks higher than one matching only one.
   - Scoring is token-based (case-insensitive). Exact matches score 1.0, partial/substring matches score 0.4, and a title match adds +3.0. Longer tokens weigh more.
   - `top_n`: number of results to return. Default is 1. Maximum is 3. Values above 3 are clamped to 3.
   - Use top_n=1 when you want the single best match. Use top_n=3 when exploring an unfamiliar topic.
   - Returns the full Markdown content of the top N matching documents, or an error string if the kb is empty or all queries are blank.
   - Output format per result: `=== Result N | kb/path/file.md | score: X.XX ===\n<contents>`

## Rules

- Always search the knowledge base BEFORE attempting to solve a problem from scratch. Prior solutions may already exist.
- Use all three query slots when approaching a problem from multiple angles (topic + technique + tool).
- Queries can be words, phrases, or full sentences.

## Usage Examples

### Example 1: Single best match
Call: `knowledge_base::search("MD5 rainbow table hash cracking", "", "", "1")`
Result: The single most relevant document.

### Example 2: Multi-angle search with broader results
Call: `knowledge_base::search("SQL injection login bypass", "Python exploit script", "CTF web challenge", "3")`
Result: Top 3 documents. Those touching multiple angles rank higher.

### Example 3: Two related topics
Call: `knowledge_base::search("RSA private key", "modular arithmetic exponent", "", "2")`
Result: Top 2 documents relevant to RSA or modular arithmetic, with overlap scoring highest.
"#;

pub fn register(f: &mut Functions, m: &mut Manual) {
    m.register("knowledge_base", MANUAL);
    f.register("knowledge_base::search", knowledge_base_search);
}

fn collect_md_files_sync(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_md_files_sync(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
            out.push(path);
        }
    }
}

fn tokenize(s: &str) -> Vec<String> {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(|t| t.to_lowercase())
        .collect()
}

fn score_query(query_tokens: &[String], doc_tokens: &[String], title_tokens: &[String]) -> f64 {
    let mut score = 0.0f64;
    for qt in query_tokens {
        if qt.is_empty() { continue; }
        let weight = (qt.len() as f64 + 1.0).log2();
        let mut best: f64 = 0.0;

        for dt in doc_tokens {
            if dt == qt {
                let s = 1.0 * weight;
                if s > best { best = s; }
            } else if dt.contains(qt.as_str()) || qt.contains(dt.as_str()) {
                let s = 0.4 * weight;
                if s > best { best = s; }
            }
        }

        for tt in title_tokens {
            if tt == qt {
                let s = (1.0 + 3.0) * weight;
                if s > best { best = s; }
            } else if tt.contains(qt.as_str()) || qt.contains(tt.as_str()) {
                let s = (0.4 + 3.0) * weight;
                if s > best { best = s; }
            }
        }

        score += best;
    }
    score
}

async fn knowledge_base_search(x: Tag) -> String {
    let q1 = x.get("q1").unwrap().as_str().to_string();
    let q2 = x.get("q2").unwrap().as_str().to_string();
    let q3 = x.get("q3").unwrap().as_str().to_string();

    let top_n = x.get("top_n")
        .map(|t| t.as_str().parse::<usize>().unwrap_or(1))
        .unwrap_or(1)
        .max(1)
        .min(3);

    let queries: Vec<&str> = [&q1, &q2, &q3]
        .iter()
        .map(|s| s.as_str())
        .filter(|s| !s.is_empty())
        .collect();

    if queries.is_empty() {
        return "no queries provided: at least one of q1, q2, q3 must be non-empty".to_string();
    }

    let tokenized_queries: Vec<Vec<String>> = queries.iter().map(|q| tokenize(q)).collect();

    let kb_path = Path::new("kb");
    if !kb_path.exists() {
        return "knowledge base directory 'kb/' does not exist".to_string();
    }

    let mut md_files: Vec<std::path::PathBuf> = Vec::new();
    collect_md_files_sync(kb_path, &mut md_files);

    if md_files.is_empty() {
        return "knowledge base is empty: no .md files found under kb/".to_string();
    }

    let mut scored: Vec<(std::path::PathBuf, f64)> = Vec::new();

    for file_path in md_files {
        let contents = match fs::read_to_string(&file_path).await {
            Ok(c) => c,
            Err(_) => continue,
        };

        let doc_tokens = tokenize(&contents);
        let stem = file_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let title_tokens = tokenize(stem);

        let mut total_score = 0.0f64;
        for qt in &tokenized_queries {
            total_score += score_query(qt, &doc_tokens, &title_tokens);
        }

        if total_score > 0.0 {
            scored.push((file_path, total_score));
        }
    }

    if scored.is_empty() {
        return "no relevant documents found for the given queries".to_string();
    }

    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut output = String::new();
    for (rank, (file_path, score)) in scored.into_iter().take(top_n).enumerate() {
        let contents = match fs::read_to_string(&file_path).await {
            Ok(c) => c,
            Err(e) => format!("(failed to read file: {})", e),
        };
        output.push_str(&format!(
            "=== Result {} | {} | score: {:.2} ===\n{}\n\n",
            rank + 1,
            file_path.display(),
            score,
            contents.trim()
        ));
    }

    output.trim_end().to_string()
}
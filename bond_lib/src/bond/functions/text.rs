use std::time::SystemTime;
use tokio::fs;
use crate::{agent::function::Functions, bond::functions::manual::Manual, taglang::Tag};

pub const MANUAL: &'static str = r#"
# Text Module Manual

This is the ONLY authorized tool for reading, creating, and modifying text-based files (code, logs, configurations).
If an operation here fails, do NOT attempt to use other tools or find "clever" workarounds (like `sed` or `echo`). Report the error.

## Functions

- text::read(path: string, a: string, b: string) -> String
   - Reads an INCLUSIVE range of lines from `a` to `b` (0-based indexing).
   - `a` and `b` are clamped automatically: `a` is clamped to `[0, last_line]`, then `b` is clamped to `[a, last_line]`.
   - CONSTRAINT: `b - a` must be less than 500. If you need more, call this function multiple times (pagination).
   - Always returns a header line before any content:
     `# size: <bytes>, lines: <total>, modified: <unix_timestamp>`
   - Content lines are formatted as: `NNN|line_content` (zero-padded line number).
   - DEFAULT BEHAVIOUR: Always start with `a="0" b="499"` — this reads up to 500 lines and also gives you the header metadata (total line count, size, timestamp) in one call. Do NOT call with `a="0" b="0"` just to inspect metadata; that wastes a round-trip. Only use a narrow range when you have a specific reason (e.g. you already know the file is large and only need a particular section).
   - If the file has more than 500 lines, paginate using subsequent calls (see pagination example below).
   - If the file is empty, only the header is returned.

- text::write(path: string, a: string, b: string, text: string, create: string) -> String
   - Replaces the inclusive range `a..=b` with the provided `text`.
   - `text` can contain multiple lines separated by '\n'.
   - To delete a range of lines, provide an empty string for `text`.
   - `create` must be either "true" or "false":
     - "true"  — creates the file (and any missing parent directories) if it does not exist, then writes `text` as the initial content with `a` and `b` both "0". If the file already exists, behaves as a normal edit using the provided `a` and `b` range.
     - "false" — edits an existing file. Fails if the file does not exist.

- text::delete(path: string) -> String
   - DANGER: This is a destructive operation. ALWAYS ask the user for confirmation before calling this.
   - Completely removes the file from the filesystem.

## Rules
- Indices `a` and `b` must be valid unsigned integers and `a` must be <= `b`.
- `b - a` must be strictly less than 500. Split into multiple calls if you need a wider range.
- `a` and `b` are clamped to valid line indices - you will never get an "invalid range" error from out-of-bounds values, but you may get fewer lines than requested if you overshoot.
- Do NOT guess line numbers. Always use `text::read` to orient yourself before writing.

## Usage Examples

### Example 1: Opening any file (default — always do this first)
Always start with a full 500-line read. You get the header metadata AND the content in one call:
Call: `text::read("main.rs", "0", "499")`
Output:
# size: 4096, lines: 312, modified: 1718000000
000|use std::io;
001|
002|fn main() {
...

The header says 312 lines total — the entire file fits within one call, no pagination needed.

### Example 2: Reading a range of lines
Call: `text::read("main.rs", "10", "12")`
Output:
# size: 4096, lines: 312, modified: 1718000000
010|fn hello() {
011|    println!("world");
012|}

### Example 3: Reading a large file (Pagination)
If the header from your first call reports more than 500 lines (e.g. 1000):
1. Call: `text::read("large.log", "0", "498")`   ← b - a = 498, within limit
2. Call: `text::read("large.log", "499", "997")`
3. Call: `text::read("large.log", "998", "999")`

### Example 4: Creating a new file
Call: `text::write("src/util.rs", "0", "0", "pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}", "true")`
Result: File does not exist → created at `src/util.rs` with the given content. Any missing parent directories are created automatically.

### Example 5: Writing to a file, creating it if absent
Call: `text::write("logs/run.log", "0", "0", "started", "true")`
Result: If `logs/run.log` exists, line 0 is replaced with "started". If it does not exist, the file is created with "started" as its content.

### Example 6: Editing lines in an existing file
To replace lines 10-12 with a new version:
Call: `text::write("main.rs", "10", "12", "fn hello() {\n    println!(\"Rust\");\n}", "false")`
Result: Lines 10, 11, and 12 are replaced by the new 3-line block.

### Example 7: Deleting a range of lines
To remove line 5 entirely:
Call: `text::write("main.rs", "5", "5", "", "false")`
Result: Line 5 is removed. The file shrinks by one line.

### Example 8: Deleting a file
Call: `text::delete("src/old.rs")`
Result: The file is permanently removed. Always confirm with the user before calling this.
"#;

pub fn register(f: &mut Functions, m: &mut Manual) {
    m.register("text", MANUAL);
    f.register("text::read", text_read);
    f.register("text::write", text_write);
    f.register("text::delete", text_delete);
}

async fn text_read(x: Tag) -> String {
    let path = x.get("path").unwrap().as_str();
    let a_raw: usize = match x.get("a").unwrap().as_str().parse() {
        Ok(n) => n,
        Err(_) => return "invalid fcall".to_string(),
    };
    let b_raw: usize = match x.get("b").unwrap().as_str().parse() {
        Ok(n) => n,
        Err(_) => return "invalid fcall".to_string(),
    };
    if a_raw > b_raw { return "invalid range: a must be <= b".to_string(); }

    let metadata = match fs::metadata(path).await {
        Ok(m) => m,
        Err(e) => return format!("failed to read file: {}", e),
    };
    let size = metadata.len();
    let modified = metadata.modified()
        .ok()
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let contents = match fs::read_to_string(path).await {
        Ok(s) => s,
        Err(e) => return format!("failed to read file: {}", e),
    };

    let lines: Vec<&str> = contents.lines().collect();
    let total = lines.len();
    let header = format!("# size: {}, lines: {}, modified: {}", size, total, modified);

    if lines.is_empty() {
        return header;
    }

    // Clamp: a to [0, last], then b to [a, last]
    let last = lines.len() - 1;
    let a = a_raw.min(last);
    let b = b_raw.min(last).max(a);

    if b - a >= 500 { return "range too large: b - a must be < 500".to_string(); }

    let width = std::cmp::max(3, b.to_string().len());
    let mut out = header;
    for idx in a..=b {
        out.push('\n');
        out.push_str(&format!("{:0width$}|{}", idx, lines[idx], width = width));
    }
    out
}

async fn text_write(x: Tag) -> String {
    let path = x.get("path").unwrap().as_str();
    let a: usize = match x.get("a").unwrap().as_str().parse() {
        Ok(n) => n,
        Err(_) => return "invalid fcall".to_string(),
    };
    let b: usize = match x.get("b").unwrap().as_str().parse() {
        Ok(n) => n,
        Err(_) => return "invalid fcall".to_string(),
    };
    let text = x.get("text").unwrap().as_str();
    let create = x.get("create").unwrap().as_str();

    if a > b { return "invalid range: a must be <= b".to_string(); }

    match create {
        "true" | "false" => {}
        _ => return "invalid fcall: create must be \"true\" or \"false\"".to_string(),
    }

    // create="true": if file is absent, create it and return immediately
    if create == "true" && fs::metadata(path).await.is_err() {
        if let Some(parent) = std::path::Path::new(path).parent() {
            if !parent.as_os_str().is_empty() {
                if let Err(e) = fs::create_dir_all(parent).await {
                    return format!("failed to create directories: {}", e);
                }
            }
        }
        return match fs::write(path, text.as_bytes()).await {
            Ok(_) => format!("created {}", path),
            Err(e) => format!("failed to create file: {}", e),
        };
    }

    // File must exist at this point (either create="false", or create="true" but file already existed)
    let existing = match fs::read_to_string(path).await {
        Ok(s) => s,
        Err(e) => return format!("failed to read file: {}", e),
    };

    let lines: Vec<String> = existing.lines().map(|s| s.to_string()).collect();
    if lines.is_empty() && a != 0 { return "invalid range".to_string(); }
    if !lines.is_empty() && (a >= lines.len() || b >= lines.len()) {
        return "invalid range".to_string();
    }

    let new_lines: Vec<String> = if text.is_empty() {
        Vec::new()
    } else {
        text.split('\n').map(|s| s.to_string()).collect()
    };

    let mut out_lines = Vec::new();
    out_lines.extend(lines.iter().take(a).cloned());
    out_lines.extend(new_lines);
    if !lines.is_empty() && b + 1 < lines.len() {
        out_lines.extend(lines.iter().skip(b + 1).cloned());
    }

    let out_text = out_lines.join("\n");
    match fs::write(path, out_text.as_bytes()).await {
        Ok(_) => format!("wrote {} bytes", out_text.len()),
        Err(e) => format!("failed to write file: {}", e),
    }
}

async fn text_delete(x: Tag) -> String {
    let path = x.get("path").unwrap().as_str();
    match fs::remove_file(path).await {
        Ok(_) => format!("deleted {}", path),
        Err(e) => format!("failed to delete file: {}", e),
    }
}
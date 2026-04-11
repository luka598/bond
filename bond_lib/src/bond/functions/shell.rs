use tokio::process::Command;

use crate::{agent::function::Functions, bond::functions::manual::Manual, taglang::Tag};

pub const MANUAL: &'static str = r#"
# Shell manual

- shell::run(cmd: string)|calls `sh -c 'cmd'` on the node. CAREFULL ALWAYS ASK BEFORE PROCEDING

Behavior:
- The function returns both stdout and stderr labeled:
  stdout: <contents>
  stderr: <contents>
- Each field is limited to 10,000 characters. If truncated, "\n...truncated" is appended to that field to indicate truncation.
- Output is decoded using UTF-8 lossy conversion so invalid bytes are replaced.

"#;

pub fn register(f: &mut Functions, m: &mut Manual) {
    m.register("shell", MANUAL);
    f.register("shell::run", shell_run);
}

fn truncate_field(s: &str, max: usize, notice: &str) -> String {
    if s.chars().count() <= max { return s.to_string(); }
    let avail = max.saturating_sub(notice.chars().count());
    let mut end_byte = s.len();
    for (i, (byte_idx, _)) in s.char_indices().enumerate() {
        if i == avail {
            end_byte = byte_idx;
            break;
        }
    }
    format!("{}{}", &s[..end_byte], notice)
}

async fn shell_run(x: Tag) -> String {
    const MAX_OUTPUT: usize = 10_000;
    const TRUNC_NOTICE: &str = "\n...truncated";

    let cmd = x.get("cmd").unwrap().as_str();

    match Command::new("sh").arg("-c").arg(cmd).output().await {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            let out_s = truncate_field(&stdout, MAX_OUTPUT, TRUNC_NOTICE);
            let err_s = truncate_field(&stderr, MAX_OUTPUT, TRUNC_NOTICE);
            format!("stdout: {}\nstderr: {}", out_s, err_s)
        }
        Err(e) => format!("failed to spawn sh: {}", e),
    }
}

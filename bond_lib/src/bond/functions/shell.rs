use std::process::Command;

const MANUAL: &str = r#"
# Shell manual
- shell::RUN(cmd: string) | calls `sh -c 'cmd'`. CAREFUL: ALWAYS ASK BEFORE PROCEEDING

Behavior:
- Returns both stdout and stderr labeled:
    stdout: <contents>
    stderr: <contents>
- Each field is limited to 10,000 characters. If truncated, "\n...truncated" is appended.
- Output is decoded using UTF-8 lossy conversion so invalid bytes are replaced.
"#;

const INFO: &str = "Runs shell commands via `sh -c`.";

pub fn call(args: &[String]) -> String {
    let (_, _, _, method, rest) = match args {
        [a, b, c, method, rest @ ..] => (a, b, c, method.as_str(), rest),
        _ => return "shell: not enough arguments".to_string(),
    };

    match method {
        "MANUAL" => MANUAL.to_string(),
        "INFO" => INFO.to_string(),
        "RUN" => {
            let cmd = match rest.first() {
                Some(c) => c.clone(),
                None => return "shell::RUN: missing cmd argument".to_string(),
            };
            run(cmd)
        }
        other => format!("shell: unknown method '{}'", other),
    }
}


fn truncate_field(s: &str, max: usize, notice: &str) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
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

fn run(cmd: String) -> String {
    const MAX_OUTPUT: usize = 10_000;
    const TRUNC_NOTICE: &str = "\n...truncated";

    match Command::new("sh").arg("-c").arg(&cmd).output() {
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

use tokio::process::Command;
use crate::{agent::function::Functions, bond::functions::manual::Manual, taglang::Tag};

pub const MANUAL: &'static str = r#"
# Tmux Module Manual

This is the ONLY authorized tool for managing terminal sessions, running background processes, and interacting with shell environments.
Use this module to decouple your "hands" (typing commands) and "eyes" (reading output) from the main execution thread.

## Functions

- tmux::is_available() -> bool
   - Checks if the `tmux` binary is installed and executable on the host system.
   - Returns `true` if available, `false` otherwise.

- tmux::create_session(session_name: string) -> void
   - Creates a new detached workspace (session) with a single default window.
   - Use this to start a clean environment.
   - Fails if a session with `session_name` already exists.

- tmux::kill_session(session_name: string) -> void
   - Destroys the specified session entirely.
   - WARNING: This kills all tabs, panes, and running processes inside that session immediately.

- tmux::list_sessions() -> Vec<String>
   - Returns a list of names of all currently active sessions.

- tmux::create_tab(session_name: string, tab_index: string) -> void
   - Opens a new "tab" (Window) inside an existing session.
   - Use this to separate different contexts (e.g., one tab for "server", one for "database").

- tmux::delete_tab(session_name: string, tab_index: string) -> void
   - Closes the specified tab and kills any processes running inside it.

- tmux::select_tab(session_name: string, tab_index: string) -> void
   - Sets the focus of the session to the specified tab.

- tmux::add_pane(session_name: string, tab_index: string) -> void
   - Splits the current view of `tab_index` to create a new pane (split-screen).
   - Useful for viewing logs while typing commands side-by-side.

- tmux::delete_pane(session_name: string, tab_index: string, pane_index: string) -> void
   - Closes a specific pane within a tab.
   - `pane_index` must be a valid integer obtained from `list_panes`.

- tmux::select_pane(session_name: string, tab_index: string, pane_index: string) -> void
   - Sets the input focus to a specific pane within a tab.

- tmux::send_keys(session_name: string, tab_index: string, pane_index: string, text: string) -> void
   - Types the provided `text` into the specific pane and automatically presses `Enter`.
   - Use this to execute shell commands.

- tmux::capture_pane(session_name: string, tab_index: string, pane_index: string) -> String
   - Returns the **visible** text content of the pane (the viewport).
   - Does NOT return the entire scrollback history. It sees what a user would see on screen.
   - Output wraps lines naturally; use this for reading immediate command output.

- tmux::get_state(session_name: string) -> String
   - Returns a structured tree view of the session.
   - Format includes: Session Name -> Tab List -> Pane List -> Last line of output for each pane.
   - Use this to get a high-level overview of what is running where.

## Rules
- `session_name` and `tab_index` are strings.
- Always ensure the session exists via `list_sessions` before interacting.
- `capture_pane` is your "eyes". It reads the current screen state.
- `send_keys` is your "hands". It inputs text.
- Do not blindly assume pane IDs; use `get_state` to verify the layout.
- If a command is long-running (like a server), use `create_tab` or `add_pane` to run it, so you don't block other interactions.

## Usage Examples

### Example 1: Setting up a fresh workspace
Call: `tmux::kill_session("dev")` (Clean up old)
Call: `tmux::create_session("dev")`
Call: `tmux::create_tab("dev", "backend")`
Result: A session "dev" exists with a default window and a "backend" window.

### Example 2: Running a command
Call: `tmux::send_keys("dev", "backend", 0, "ls -la")`
Result: The `ls -la` command runs in the first pane of the "backend" tab.

### Example 3: Reading the result
Call: `tmux::capture_pane("dev", "backend", 0)`
Output:
total 16
drwxr-xr-x  3 user user 4096 Feb 17 12:00 .
drwxr-xr-x 10 user user 4096 Feb 17 11:00 ..
-rw-r--r--  1 user user  120 Feb 17 12:00 main.rs

### Example 4: Complex Multi-tasking
1. Call: `tmux::add_pane("dev", "backend")` (Splits screen).
2. Call: `tmux::send_keys("dev", "backend", 1, "tail -f server.log")` (Watch logs in pane 1).
3. Call: `tmux::send_keys("dev", "backend", 0, "curl localhost:8080")` (Make request in pane 0).
4. Call: `tmux::get_state("dev")`
Output:
Session: dev
  [Tab: backend] -> [Pane: 0] (Running: curl)
      Last Line: "HTTP/1.1 200 OK"
  [Tab: backend] -> [Pane: 1] (Running: tail)
      Last Line: "[INFO] Incoming request..."


"#;

pub fn register(f: &mut Functions, m: &mut Manual) {
    m.register("tmux", MANUAL);
    f.register("tmux::is_available", tmux_is_available);
    f.register("tmux::create_session", tmux_create_session);
    f.register("tmux::kill_session", tmux_kill_session);
    f.register("tmux::list_sessions", tmux_list_sessions);
    f.register("tmux::create_tab", tmux_create_tab);
    f.register("tmux::delete_tab", tmux_delete_tab);
    f.register("tmux::select_tab", tmux_select_tab);
    f.register("tmux::add_pane", tmux_add_pane);
    f.register("tmux::delete_pane", tmux_delete_pane);
    f.register("tmux::select_pane", tmux_select_pane);
    f.register("tmux::send_keys", tmux_send_keys);
    f.register("tmux::capture_pane", tmux_capture_pane);
    f.register("tmux::get_state", tmux_get_state);
}

async fn run_void(args: &[&str]) -> String {
    match Command::new("tmux").args(args).status().await {
        Ok(s) if s.success() => "ok".to_string(),
        Ok(s) => format!("failed with status: {}", s),
        Err(e) => format!("execution error: {}", e),
    }
}

async fn run_out(args: &[&str]) -> Result<String, String> {
    let output = Command::new("tmux").args(args).output().await
        .map_err(|e| format!("execution error: {}", e))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

async fn tmux_is_available(_x: Tag) -> String {
    match Command::new("tmux").arg("-V").output().await {
        Ok(out) if out.status.success() => "true".to_string(),
        _ => "false".to_string(),
    }
}

async fn tmux_create_session(x: Tag) -> String {
    let session = x.get("session_name").unwrap().as_str();
    run_void(&["new-session", "-d", "-s", session]).await
}

async fn tmux_kill_session(x: Tag) -> String {
    let session = x.get("session_name").unwrap().as_str();
    run_void(&["kill-session", "-t", session]).await
}

async fn tmux_list_sessions(_x: Tag) -> String {
    match run_out(&["list-sessions", "-F", "#{session_name}"]).await {
        Ok(out) => if out.is_empty() { "(none)".to_string() } else { out },
        Err(e) => format!("error: {}", e),
    }
}

async fn tmux_create_tab(x: Tag) -> String {
    let session = x.get("session_name").unwrap().as_str();
    let title = x.get("title").unwrap().as_str();
    run_void(&["new-window", "-t", session, "-n", title]).await
}

async fn tmux_delete_tab(x: Tag) -> String {
    let session = x.get("session_name").unwrap().as_str();
    let tab = x.get("tab_index").unwrap().as_str();
    run_void(&["kill-window", "-t", &format!("{}:{}", session, tab)]).await
}

async fn tmux_select_tab(x: Tag) -> String {
    let session = x.get("session_name").unwrap().as_str();
    let tab = x.get("tab_index").unwrap().as_str();
    run_void(&["select-window", "-t", &format!("{}:{}", session, tab)]).await
}

async fn tmux_add_pane(x: Tag) -> String {
    let session = x.get("session_name").unwrap().as_str();
    let tab = x.get("tab_index").unwrap().as_str();
    run_void(&["split-window", "-t", &format!("{}:{}", session, tab)]).await
}

async fn tmux_delete_pane(x: Tag) -> String {
    let session = x.get("session_name").unwrap().as_str();
    let tab = x.get("tab_index").unwrap().as_str();
    let pane = x.get("pane_index").unwrap().as_str();
    run_void(&["kill-pane", "-t", &format!("{}:{}.{}", session, tab, pane)]).await
}

async fn tmux_select_pane(x: Tag) -> String {
    let session = x.get("session_name").unwrap().as_str();
    let tab = x.get("tab_index").unwrap().as_str();
    let pane = x.get("pane_index").unwrap().as_str();
    run_void(&["select-pane", "-t", &format!("{}:{}.{}", session, tab, pane)]).await
}

async fn tmux_send_keys(x: Tag) -> String {
    let session = x.get("session_name").unwrap().as_str();
    let tab = x.get("tab_index").unwrap().as_str();
    let pane = x.get("pane_index").unwrap().as_str();
    let text = x.get("text").unwrap().as_str();
    run_void(&["send-keys", "-t", &format!("{}:{}.{}", session, tab, pane), text, "Enter"]).await
}

async fn tmux_capture_pane(x: Tag) -> String {
    let session = x.get("session_name").unwrap().as_str();
    let tab = x.get("tab_index").unwrap().as_str();
    let pane = x.get("pane_index").unwrap().as_str();
    match run_out(&["capture-pane", "-pt", &format!("{}:{}.{}", session, tab, pane), "-J"]).await {
        Ok(out) => out,
        Err(e) => e,
    }
}

async fn tmux_get_state(x: Tag) -> String {
    let session = x.get("session_name").unwrap().as_str();
    let format = "#{window_index}|#{window_name}|#{pane_index}|#{pane_current_command}";
    let panes_raw = match run_out(&["list-panes", "-s", "-t", session, "-F", format]).await {
        Ok(out) => out,
        Err(e) => return format!("failed to get state: {}", e),
    };

    let mut report = format!("Session: {}\n", session);
    for line in panes_raw.lines() {
        let p: Vec<&str> = line.split('|').collect();
        if p.len() < 4 { continue; }
        let target = format!("{}:{}.{}", session, p[0], p[2]);
        let last = run_out(&["capture-pane", "-pt", &target, "-S", "-1"]).await.unwrap_or_default();
        report.push_str(&format!(
            "  [Tab Index: {}] (Title: {}) > [Pane Index: {}] (Cmd: {})\n    Last Line: \"{}\"\n",
            p[0], p[1], p[2], p[3], last.trim()
        ));
    }
    report
}
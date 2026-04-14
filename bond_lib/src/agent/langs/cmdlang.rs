#[derive(Debug)]
pub struct Cmd {
    pub fname: String,
    pub args: Vec<String>,
}

impl Cmd {
    pub fn parse(cmd: &str) -> Result<Self, String> {
        let (mut pos, delim) = Self::parse_quote(0, cmd)?;
        let fname;
        (pos, fname) = Self::parse_fname(pos, cmd)?;

        let mut args = Vec::new();
        while pos < cmd.len() {
            let skipped = cmd[pos..].len() - cmd[pos..].trim_start_matches([' ', '\t', '\n']).len();
            pos += skipped;
            if pos >= cmd.len() {
                break;
            }
            let arg;
            (pos, arg) = Self::parse_arg(pos, cmd, &delim)?;
            args.push(arg);
        }

        Ok(Self { fname, args })
    }

    fn parse_quote(pos: usize, cmd: &str) -> Result<(usize, String), String> {
        if let Some(rest) = cmd.strip_prefix("QUOTE=") {
            let space = rest
                .find(' ')
                .ok_or("QUOTE= directive missing trailing space")?;
            return Ok((6 + space + 1, rest[..space].to_string()));
        }
        Ok((pos, "\"".to_string()))
    }

    fn parse_fname(pos: usize, cmd: &str) -> Result<(usize, String), String> {
        if pos >= cmd.len() {
            return Err("expected function name".to_string());
        }
        let rest = &cmd[pos..];
        let space = rest.find(' ').unwrap_or(rest.len());
        Ok((pos + space + 1, rest[..space].to_string()))
    }

    fn parse_arg(pos: usize, cmd: &str, delim: &str) -> Result<(usize, String), String> {
        if !cmd[pos..].starts_with(delim) {
            let got = cmd[pos..].chars().next().unwrap_or('\0');
            return Err(format!(
                "char {pos}: expected opening {delim:?}, got {got:?}"
            ));
        }
        let open_pos = pos;
        let pos = pos + delim.len();
        let close = cmd[pos..]
            .find(delim)
            .map(|i| i + pos)
            .ok_or_else(|| format!("char {open_pos}: unclosed {delim:?}"))?;
        Ok((close + delim.len(), cmd[pos..close].to_string()))
    }
}

pub const CMDLANG_PROMPT: &str = r#"
# CmdLang

Commands follow this format:
```
fname "arg1" "arg2" "arg3"
```

`fname` is the function name (no spaces), followed by zero or more arguments each wrapped in the delimiter (default: `"`).

## Arguments are Positional

CmdLang has **no named arguments**. Arguments are passed by position — order matters. The function definition determines what each position means. Do not write `fname key="value"` or anything like it; just supply values in the correct order.

```
# WRONG
move x="10" y="20"

# CORRECT
move "10" "20"
```

### Basic Examples
```
greet "hello" "world"
add "42" "7"
no_args
```

## Changing the Delimiter (`QUOTE=`)

The default delimiter is `"`. If any argument value **contains a literal `"`**, you must switch to a different delimiter using the `QUOTE=` prefix before the function name:

```
QUOTE=| fname |arg with "quotes" inside|
QUOTE=### fname ###first arg### ###second arg###
```

The delimiter can be any string that does **not appear** in any of your argument values. Rules:
- `QUOTE=<delim> <fname> <args...>` — the delimiter ends at the first space after `QUOTE=`
- Every argument must be wrapped: `<delim>value<delim>`
- The delimiter applies to **all** args in the call

## Choosing a Safe Delimiter

Scan your argument values and pick a string absent from all of them:

| Situation | Suggested delimiter |
|---|---|
| Args contain `"` | `\|` or `'` |
| Args contain `"` and `\|` | `###` or `~~~` |
| Args contain arbitrary text/code | A long unique string like `%%END%%` |

## Edge Cases

- **No args:** just `fname` with no delimiter needed
- **Empty arg:** `fname "" ""` — empty string between delimiters is valid
- **Spaces in args:** fine, delimiters handle it — `fname "hello world"`
- **Delimiter must not appear mid-arg** — if it does, the parser will close the argument early and produce a wrong parse or error

## Errors You May Receive

- `expected opening "X"` — you forgot to open an arg with the delimiter
- `unclosed "X"` — you opened an arg but never closed it
- `expected function name` — the command string was empty
"#;
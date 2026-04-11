// === Tag

#[derive(Debug, Clone, PartialEq)]
pub struct Tag {
    pub name: String,
    pub suffix: String,
    pub value: String,
    pub children: Vec<Tag>,
}

impl Tag {
    fn new(name: impl Into<String>, suffix: impl Into<String>) -> Self {
        Tag {
            name: name.into(),
            suffix: suffix.into(),
            value: String::new(),
            children: Vec::new(),
        }
    }

    pub fn get(&self, name: &str) -> Option<&Tag> {
        self.children.iter().find(|c| c.name == name)
    }

    pub fn get_all(&self, name: &str) -> Vec<&Tag> {
        self.children.iter().filter(|c| c.name == name).collect()
    }

    pub fn as_str(&self) -> &str {
        let mut value = self.value.trim();
        if value.len() == 0 {
           value = self.get("raw").and_then(|x| Some(x.as_str())).unwrap_or("") ;
        }

        value

    }

    pub fn as_bool(&self) -> bool {
        matches!(self.as_str(), "true" | "1" | "yes")
    }

    pub fn as_i64(&self) -> Option<i64> {
        self.as_str().parse().ok()
    }

    pub fn as_f64(&self) -> Option<f64> {
        self.as_str().parse().ok()
    }
}

// === Parser

pub fn parse(input: &str) -> Result<Tag, String> {
    let mut cursor = Cursor::new(input);
    let mut root = Tag::new("root", "");

    loop {
        cursor.skip_whitespace();
        if !cursor.has_remaining() {
            break;
        }

        if cursor.starts_with("</") {
            // Close tag at root level — nothing is open.
            let (name, suffix) = cursor.read_close_tag()?;
            return Err(format!(
                "Unmatched {} tag",
                tag_display(false, &name, &suffix)
            ));
        }

        if cursor.peek() == Some('<') {
            let child = parse_tag(&mut cursor)?;
            root.children.push(child);
        } else {
            let text = cursor.read_text();
            if !text.trim().is_empty() {
                root.value.push_str(text);
            }
        }
    }

    Ok(root)
}

fn parse_tag(cursor: &mut Cursor) -> Result<Tag, String> {
    let (name, suffix) = cursor.read_open_tag()?;

    if name == "raw" {
        if suffix.is_empty() {
            return Err(format!(
                "<raw> tag requires a suffix (e.g. <raw EOF>) at position {}",
                cursor.pos
            ));
        }
        return parse_raw_tag(cursor, &suffix);
    }

    let mut tag = Tag::new(&name, &suffix);

    loop {
        if !cursor.has_remaining() {
            return Err(format!(
                "Unmatched {} tag",
                tag_display(true, &name, &suffix)
            ));
        }

        if cursor.starts_with("</") {
            let saved = cursor.pos;
            let (close_name, close_suffix) = cursor.read_close_tag()?;

            if close_name == name && close_suffix == suffix {
                // Correct closer - done.
                break;
            }

            // Mismatched close tag → raw text.
            cursor.pos = saved;
            let raw = cursor.read_until_gt();
            tag.value.push_str(&raw);
            continue;
        }

        if cursor.peek() == Some('<') {
            let child = parse_tag(cursor)?;
            tag.children.push(child);
        } else {
            let text = cursor.read_text();
            tag.value.push_str(text);
        }
    }

    tag.value = tag.value.trim().to_string();
    Ok(tag)
}

/// Capture everything up to and including `</raw SUFFIX>` as a raw string.
fn parse_raw_tag(cursor: &mut Cursor, suffix: &str) -> Result<Tag, String> {
    // The close marker we are scanning for: </raw SUFFIX>
    // We build it as a byte pattern and scan manually (no regex).
    let close_marker = format!("</raw {suffix}>");
    let marker_bytes = close_marker.as_bytes();

    let start = cursor.pos;

    loop {
        if cursor.pos + marker_bytes.len() > cursor.src.len() {
            return Err(format!(
                "Unmatched {} tag",
                tag_display(true, "raw", suffix)
            ));
        }
        if cursor.src[cursor.pos..].starts_with(marker_bytes) {
            let raw_value =
                std::str::from_utf8(&cursor.src[start..cursor.pos]).unwrap().to_string();
            cursor.pos += marker_bytes.len(); // consume the close marker
            let mut tag = Tag::new("raw", suffix);
            tag.value = raw_value;
            return Ok(tag);
        }
        cursor.pos += 1;
    }
}

// === Cursor

struct Cursor<'a> {
    src: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(input: &'a str) -> Self {
        Cursor { src: input.as_bytes(), pos: 0 }
    }

    fn has_remaining(&self) -> bool {
        self.pos < self.src.len()
    }

    fn peek(&self) -> Option<char> {
        self.src.get(self.pos).map(|&b| b as char)
    }

    fn starts_with(&self, s: &str) -> bool {
        self.src[self.pos..].starts_with(s.as_bytes())
    }

    fn advance(&mut self) {
        if self.pos < self.src.len() { self.pos += 1; }
    }

    fn skip_whitespace(&mut self) {
        while self.has_remaining() && self.src[self.pos].is_ascii_whitespace() {
            self.advance();
        }
    }

    fn read_tag_name(&mut self) -> Result<String, String> {
        let start = self.pos;
        while self.has_remaining() {
            let b = self.src[self.pos];
            if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' {
                self.advance();
            } else {
                break;
            }
        }
        if self.pos == start {
            return Err(format!(
                "Expected tag name at position {}, found {:?}",
                self.pos, self.peek()
            ));
        }
        Ok(std::str::from_utf8(&self.src[start..self.pos]).unwrap().to_string())
    }

    /// Read optional suffix: whitespace-separated token(s) before `>`.
    /// Everything between the first whitespace and `>` is the suffix (trimmed).
    fn read_optional_suffix(&mut self) -> Result<String, String> {
        self.skip_whitespace();
        if self.peek() == Some('>') {
            return Ok(String::new());
        }
        let start = self.pos;
        while self.has_remaining() && self.src[self.pos] != b'>' {
            self.advance();
        }
        Ok(std::str::from_utf8(&self.src[start..self.pos])
            .unwrap()
            .trim()
            .to_string())
    }

    /// Consume `<name[ suffix]>` → `(name, suffix)`.
    fn read_open_tag(&mut self) -> Result<(String, String), String> {
        self.expect('<')?;
        let name = self.read_tag_name()?;
        let suffix = self.read_optional_suffix()?;
        self.expect('>')?;
        Ok((name, suffix))
    }

    /// Consume `</name[ suffix]>` → `(name, suffix)`.
    fn read_close_tag(&mut self) -> Result<(String, String), String> {
        self.expect('<')?;
        self.expect('/')?;
        let name = self.read_tag_name()?;
        let suffix = self.read_optional_suffix()?;
        self.expect('>')?;
        Ok((name, suffix))
    }

    /// Read text up to (not including) the next `<`.
    fn read_text(&mut self) -> &'a str {
        let start = self.pos;
        while self.has_remaining() && self.src[self.pos] != b'<' {
            self.advance();
        }
        std::str::from_utf8(&self.src[start..self.pos]).unwrap()
    }

    /// Read from current position up to and including `>`, returning the whole
    /// slice as a string (used to re-emit a mismatched close tag as raw text).
    fn read_until_gt(&mut self) -> String {
        let start = self.pos;
        while self.has_remaining() && self.src[self.pos] != b'>' {
            self.advance();
        }
        if self.has_remaining() { self.advance(); } // consume `>`
        std::str::from_utf8(&self.src[start..self.pos]).unwrap().to_string()
    }

    fn expect(&mut self, ch: char) -> Result<(), String> {
        match self.peek() {
            Some(c) if c == ch => { self.advance(); Ok(()) }
            other => Err(format!(
                "Expected '{ch}' at position {}, found {:?}", self.pos, other
            )),
        }
    }
}

// === util

fn tag_display(open: bool, name: &str, suffix: &str) -> String {
    let slash = if open { "" } else { "/" };
    if suffix.is_empty() {
        format!("<{slash}{name}>")
    } else {
        format!("<{slash}{name} {suffix}>")
    }
}
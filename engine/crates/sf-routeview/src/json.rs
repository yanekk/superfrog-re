//! Just enough JSON to write the page's data: `serde_json` is not in the workspace (sf-bench
//! `results.rs` reads and writes its lines by hand for the same reason).

/// `s` as the inside of a JSON string. `</` is written `<\/`, so the text can sit inside a
/// `<script>` element without closing it.
pub fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            '/' if o.ends_with('<') => o.push_str("\\/"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o
}

/// An object written field by field.
pub struct Json(String);

impl Json {
    pub fn object() -> Json {
        Json(String::from("{"))
    }

    fn key(&mut self, k: &str) {
        if self.0.len() > 1 {
            self.0.push(',');
        }
        self.0.push('"');
        self.0.push_str(&esc(k));
        self.0.push_str("\":");
    }

    pub fn str(&mut self, k: &str, v: &str) {
        self.key(k);
        self.0.push('"');
        self.0.push_str(&esc(v));
        self.0.push('"');
    }

    pub fn num(&mut self, k: &str, v: impl std::fmt::Display) {
        self.key(k);
        self.0.push_str(&v.to_string());
    }

    pub fn opt(&mut self, k: &str, v: Option<impl std::fmt::Display>) {
        self.key(k);
        self.0.push_str(&v.map_or("null".to_string(), |v| v.to_string()));
    }

    pub fn bool(&mut self, k: &str, v: bool) {
        self.key(k);
        self.0.push_str(if v { "true" } else { "false" });
    }

    /// `v` must already be JSON.
    pub fn raw(&mut self, k: &str, v: &str) {
        self.key(k);
        self.0.push_str(v);
    }

    pub fn finish(mut self) -> String {
        self.0.push('}');
        self.0
    }
}

/// A list of already-JSON items.
pub fn list(items: impl Iterator<Item = String>) -> String {
    format!("[{}]", items.collect::<Vec<_>>().join(","))
}

/// A list of strings.
pub fn strs<'a>(items: impl Iterator<Item = &'a String>) -> String {
    list(items.map(|s| format!("\"{}\"", esc(s))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_quotes_controls_and_script_ends() {
        assert_eq!(esc("a\"b\\c\nd"), "a\\\"b\\\\c\\nd");
        assert_eq!(esc("</script>"), "<\\/script>");
        assert_eq!(esc("1/2"), "1/2");
        assert_eq!(esc("\u{1}"), "\\u0001");
    }

    #[test]
    fn objects_join_fields() {
        let mut j = Json::object();
        j.str("a", "x");
        j.num("b", 3);
        j.opt("c", None::<u32>);
        j.bool("d", true);
        j.raw("e", "[1,2]");
        assert_eq!(j.finish(), r#"{"a":"x","b":3,"c":null,"d":true,"e":[1,2]}"#);
    }
}

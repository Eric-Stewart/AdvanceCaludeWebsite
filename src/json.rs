//! A tiny JSON writer. We have no external crates, and hand-concatenating
//! strings would eventually ship a response broken by an unescaped backslash —
//! which, with this much LaTeX flying around, is a certainty rather than a risk.

use std::fmt::Write as _;

pub enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Int(i128),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    pub fn str(s: impl Into<String>) -> Json {
        Json::Str(s.into())
    }

    pub fn obj<const N: usize>(fields: [(&str, Json); N]) -> Json {
        Json::Obj(fields.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
    }

    pub fn arr(items: impl IntoIterator<Item = Json>) -> Json {
        Json::Arr(items.into_iter().collect())
    }

    pub fn to_string(&self) -> String {
        let mut out = String::new();
        self.write(&mut out);
        out
    }

    fn write(&self, out: &mut String) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Int(n) => {
                let _ = write!(out, "{n}");
            }
            Json::Num(x) => {
                if x.is_finite() {
                    let _ = write!(out, "{x}");
                } else {
                    out.push_str("null"); // JSON has no NaN/Infinity
                }
            }
            Json::Str(s) => escape_into(s, out),
            Json::Arr(items) => {
                out.push('[');
                for (i, v) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    v.write(out);
                }
                out.push(']');
            }
            Json::Obj(fields) => {
                out.push('{');
                for (i, (k, v)) in fields.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    escape_into(k, out);
                    out.push(':');
                    v.write(out);
                }
                out.push('}');
            }
        }
    }
}

fn escape_into(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            // Control characters must be escaped; U+2028/2029 are legal JSON but
            // break when a response is ever inlined into a <script> tag.
            c if (c as u32) < 0x20 || c == '\u{2028}' || c == '\u{2029}' => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_latex_backslashes() {
        let j = Json::str(r"\frac{1}{2} \\ \alpha");
        assert_eq!(j.to_string(), r#""\\frac{1}{2} \\\\ \\alpha""#);
    }

    #[test]
    fn escapes_quotes_and_controls() {
        assert_eq!(Json::str("a\"b").to_string(), r#""a\"b""#);
        assert_eq!(Json::str("a\nb\tc").to_string(), r#""a\nb\tc""#);
        assert_eq!(Json::str("\u{1}").to_string(), r#""\u0001""#);
        assert_eq!(Json::str("\u{2028}").to_string(), r#""\u2028""#);
    }

    #[test]
    fn nested_structures() {
        let j = Json::obj([
            ("ok", Json::Bool(true)),
            ("n", Json::Int(-7)),
            ("xs", Json::arr([Json::Int(1), Json::Int(2)])),
            ("nil", Json::Null),
        ]);
        assert_eq!(j.to_string(), r#"{"ok":true,"n":-7,"xs":[1,2],"nil":null}"#);
    }

    #[test]
    fn non_finite_numbers_become_null() {
        assert_eq!(Json::Num(f64::NAN).to_string(), "null");
        assert_eq!(Json::Num(f64::INFINITY).to_string(), "null");
        assert_eq!(Json::Num(1.5).to_string(), "1.5");
    }

    #[test]
    fn empty_containers() {
        assert_eq!(Json::arr([]).to_string(), "[]");
        assert_eq!(Json::obj([]).to_string(), "{}");
    }
}

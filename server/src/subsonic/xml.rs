//! JSON の応答から XML の応答を作る。
//!
//! Subsonic の XML は、スカラーを属性に、オブジェクトを子要素に、配列を同名の子要素の繰り返しにする。
//! 例外として、`value` という文字列は要素の本文にする（歌詞などで使う）。

use std::fmt::Write;

use serde_json::{Map, Value};

const TEXT_KEY: &str = "value";

pub fn document(root: &str, body: &Map<String, Value>) -> String {
    let mut out = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    write_object(&mut out, root, body);
    out
}

fn write_element(out: &mut String, name: &str, value: &Value) {
    match value {
        Value::Null => {}
        Value::Object(map) => write_object(out, name, map),
        Value::Array(items) => {
            for item in items {
                write_element(out, name, item);
            }
        }
        scalar => {
            let _ = write!(out, "<{name}>{}</{name}>", escape(&scalar_text(scalar)));
        }
    }
}

fn write_object(out: &mut String, name: &str, map: &Map<String, Value>) {
    let _ = write!(out, "<{name}");
    let mut text = None;
    for (key, value) in map {
        match value {
            Value::Null | Value::Object(_) | Value::Array(_) => {}
            Value::String(s) if key == TEXT_KEY => text = Some(s.as_str()),
            scalar => {
                let _ = write!(out, r#" {key}="{}""#, escape(&scalar_text(scalar)));
            }
        }
    }
    let children: Vec<_> = map
        .iter()
        .filter(|(_, v)| v.is_object() || v.is_array())
        .collect();
    if children.is_empty() && text.is_none() {
        out.push_str("/>");
        return;
    }
    out.push('>');
    if let Some(text) = text {
        out.push_str(&escape(text));
    }
    for (key, value) in children {
        write_element(out, key, value);
    }
    let _ = write!(out, "</{name}>");
}

fn scalar_text(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn render(value: Value) -> String {
        let Value::Object(map) = value else {
            unreachable!()
        };
        document("r", &map)
            .trim_start_matches(r#"<?xml version="1.0" encoding="UTF-8"?>"#)
            .to_owned()
    }

    #[test]
    fn scalars_become_attributes() {
        assert_eq!(
            render(json!({ "status": "ok", "count": 3, "valid": true })),
            r#"<r count="3" status="ok" valid="true"/>"#
        );
    }

    #[test]
    fn arrays_repeat_elements() {
        assert_eq!(
            render(json!({ "ext": [
                { "name": "formPost", "versions": [1] },
                { "name": "apiKeyAuthentication", "versions": [1] },
            ]})),
            r#"<r><ext name="formPost"><versions>1</versions></ext><ext name="apiKeyAuthentication"><versions>1</versions></ext></r>"#
        );
    }

    #[test]
    fn value_becomes_text_and_is_escaped() {
        assert_eq!(
            render(json!({ "line": { "start": 0, "value": "a & <b>" } })),
            r#"<r><line start="0">a &amp; &lt;b&gt;</line></r>"#
        );
    }

    #[test]
    fn nulls_are_omitted() {
        assert_eq!(render(json!({ "a": null, "b": 1 })), r#"<r b="1"/>"#);
    }
}

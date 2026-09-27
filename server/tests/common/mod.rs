//! 結合テストで共通に使う、Navidrome の応答（`tests/fixtures/navidrome/`）との比べ方。
//! 値ではなく、応答の要素名と、各要素が持つ項目の名前と型を比べる。

use std::fs;

use serde_json::{Map, Value};

const ENVELOPE: [&str; 5] = ["status", "version", "type", "serverVersion", "openSubsonic"];

pub fn navidrome(name: &str) -> Map<String, Value> {
    let path = format!(
        "{}/tests/fixtures/navidrome/{name}.json",
        env!("CARGO_MANIFEST_DIR")
    );
    payload(serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap())
}

/// 共通の項目を除いた、応答の中身。
pub fn payload(value: Value) -> Map<String, Value> {
    let Value::Object(mut res) = value["subsonic-response"].clone() else {
        panic!("not a subsonic response: {value}");
    };
    assert_eq!(res["status"], "ok", "{res:?}");
    res.retain(|k, _| !ENVELOPE.contains(&k.as_str()));
    res
}

fn kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

/// `ours` の項目がすべて `theirs` にあり、型が同じであることを確かめる。
pub fn assert_subset(path: &str, ours: &Value, theirs: &Value) {
    assert_eq!(kind(ours), kind(theirs), "{path}");
    match (ours, theirs) {
        (Value::Object(ours), Value::Object(theirs)) => {
            for (key, value) in ours {
                let other = theirs
                    .get(key)
                    .unwrap_or_else(|| panic!("{path}.{key} is not in Navidrome's response"));
                assert_subset(&format!("{path}.{key}"), value, other);
            }
        }
        (Value::Array(ours), Value::Array(theirs)) => {
            if let (Some(ours), Some(theirs)) = (ours.first(), theirs.first()) {
                assert_subset(&format!("{path}[]"), ours, theirs);
            }
        }
        _ => {}
    }
}

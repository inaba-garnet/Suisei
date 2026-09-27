//! 同一判定に使う鍵（docs/schema.md の「ID と同一判定」）。

use unicode_normalization::UnicodeNormalization;

/// 欄の区切り。正規化で制御文字を落とすので、値の中には現れない
const FIELD_SEP: char = '\u{1f}';
/// 一つの欄に複数の名前を並べるときの区切り
const LIST_SEP: char = '\u{1e}';

/// NFKC 正規化、小文字への統一、空白の整理をする。
pub fn normalize(value: &str) -> String {
    let folded = value.nfkc().collect::<String>().to_lowercase();
    // タブや改行は空白として扱い、それ以外の制御文字を落とす
    folded
        .split_whitespace()
        .map(|word| word.chars().filter(|c| !c.is_control()).collect::<String>())
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// 正規化済みの欄をつないで鍵にする。
pub(super) fn join<'a>(fields: impl IntoIterator<Item = &'a str>) -> String {
    let mut key = String::new();
    for (i, field) in fields.into_iter().enumerate() {
        if i > 0 {
            key.push(FIELD_SEP);
        }
        key.push_str(field);
    }
    key
}

/// 正規化済みの名前を並べて一つの欄にする。
pub(super) fn list<'a>(names: impl IntoIterator<Item = &'a str>) -> String {
    let mut field = String::new();
    for (i, name) in names.into_iter().enumerate() {
        if i > 0 {
            field.push(LIST_SEP);
        }
        field.push_str(name);
    }
    field
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn width_and_case() {
        // fixture の表記ゆれ（ClariS、LiSA）と同じ種類のゆれ
        assert_eq!(normalize("ＣｌａｒｉＳ"), normalize("ClariS"));
        assert_eq!(normalize("RISING HOPE"), normalize("Rising Hope"));
        assert_eq!(normalize("ｶﾑﾊﾟﾈﾙﾗ"), "カムパネルラ");
    }

    #[test]
    fn whitespace() {
        assert_eq!(normalize("  感電 "), "感電");
        assert_eq!(normalize("米津\u{3000}\u{3000}玄師"), "米津 玄師");
        assert_eq!(normalize("a\tb\nc"), "a b c");
    }

    #[test]
    fn control_chars_are_dropped() {
        assert_eq!(normalize("a\u{1f}b"), "ab");
    }

    #[test]
    fn fields_do_not_shift() {
        assert_ne!(join(["a b", "c"]), join(["a", "b c"]));
        assert_ne!(list(["a", "b"]), join(["a", "b"]));
    }
}

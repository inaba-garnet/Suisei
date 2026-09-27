//! 検索で照らす文字列。

use wana_kana::utils::hiragana_to_katakana::hiragana_to_katakana;

use super::normalize;

/// 欄の区切り。検索語には現れないので、語が欄をまたいで一致しない
const SEP: char = '\u{1f}';

/// 検索語と照らす文字列にそろえる。`match_key` と同じ正規化に加え、ひらがなをカタカナにする。
pub fn search_form(value: &str) -> String {
    hiragana_to_katakana(&normalize(value))
}

/// 欄をつないで、照らす文字列にする。空白を含む欄は、空白を除いた形も足す。
/// 読みの `ヨネズ ケンシ` を `よねずけんし` でも見つけられるようにするため。
pub fn search_text<'a>(fields: impl IntoIterator<Item = &'a str>) -> String {
    let mut text = String::new();
    for field in fields {
        let form = search_form(field);
        if form.is_empty() {
            continue;
        }
        let joined = form.replace(' ', "");
        for part in [form.as_str(), joined.as_str()] {
            if !text.is_empty() {
                text.push(SEP);
            }
            text.push_str(part);
            if joined == form {
                break;
            }
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hiragana_matches_reading() {
        let text = search_text(["米津玄師", "ヨネズ ケンシ"]);
        assert!(text.contains(&search_form("よねず")));
        assert!(text.contains(&search_form("よねずけんし")));
        assert!(text.contains(&search_form("ヨネズ ケンシ")));
    }

    #[test]
    fn width_and_case_are_folded() {
        let text = search_text(["ＣｌａｒｉＳ"]);
        assert!(text.contains(&search_form("claris")));
    }

    #[test]
    fn fields_are_separated() {
        let text = search_text(["ab", "cd"]);
        assert!(!text.contains(&search_form("bc")));
    }
}

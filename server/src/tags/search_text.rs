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

/// アーティストの照らす文字列。名前と読み。
pub fn artist_text(name: &str, reading: Option<&str>) -> String {
    search_text([name, reading.unwrap_or_default()])
}

/// アルバムの照らす文字列。名前と読み、表示用アーティスト、アルバムアーティストの読み。
pub fn album_text<'a>(
    name: &'a str,
    reading: Option<&'a str>,
    display_artist: &'a str,
    artist_readings: impl IntoIterator<Item = &'a str>,
) -> String {
    search_text(
        [name, reading.unwrap_or_default(), display_artist]
            .into_iter()
            .chain(artist_readings),
    )
}

/// 曲の照らす文字列。曲名と読み、表示用アーティスト、アルバム名、曲のアーティストの読み。
pub fn track_text<'a>(
    title: &'a str,
    reading: Option<&'a str>,
    display_artist: &'a str,
    album: &'a str,
    artist_readings: impl IntoIterator<Item = &'a str>,
) -> String {
    search_text(
        [title, reading.unwrap_or_default(), display_artist, album]
            .into_iter()
            .chain(artist_readings),
    )
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

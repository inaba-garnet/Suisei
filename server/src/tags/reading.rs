//! 読みの規則（docs/schema.md の「日本語の並べ替え」）。
//! 手動設定と形態素解析はここでは扱わない。

use unicode_normalization::UnicodeNormalization;
use wana_kana::ConvertJapanese;
use wana_kana::utils::hiragana_to_katakana::hiragana_to_katakana;

/// カタカナの読みと、どこから得たか。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reading {
    pub kana: String,
    pub source: ReadingSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadingSource {
    SortTag,
    Name,
}

impl ReadingSource {
    /// DB の `sort_name_source` に入れる値
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SortTag => "sort_tag",
            Self::Name => "name",
        }
    }
}

/// 名前とソート用タグから読みを決める。
pub fn reading(name: &str, sort_tag: Option<&str>) -> Option<Reading> {
    let from_sort_tag = sort_tag.and_then(|sort| {
        let sort: String = sort.nfkc().collect();
        if is_kana_text(&sort) {
            Some(hiragana_to_katakana(&sort))
        } else if contains_japanese(name) {
            romaji_to_katakana(&sort)
        } else {
            None
        }
    });
    if let Some(kana) = from_sort_tag {
        return Some(Reading {
            kana,
            source: ReadingSource::SortTag,
        });
    }
    let name: String = name.nfkc().collect();
    is_kana_text(&name).then(|| Reading {
        kana: hiragana_to_katakana(&name),
        source: ReadingSource::Name,
    })
}

/// かなと記号だけでできているか。数字は記号に含めない。
fn is_kana_text(text: &str) -> bool {
    text.chars().any(is_kana) && text.chars().all(|c| is_kana(c) || !c.is_alphanumeric())
}

fn is_kana(c: char) -> bool {
    // ひらがなとカタカナの区画。長音記号（ー）と踊り字（ゝ、ヽ）もここに入る
    matches!(c, '\u{3041}'..='\u{309f}' | '\u{30a0}'..='\u{30ff}')
}

fn contains_japanese(text: &str) -> bool {
    text.chars().any(|c| {
        is_kana(c)
            || matches!(c,
                '\u{3005}' // 々
                | '\u{3400}'..='\u{4dbf}'
                | '\u{4e00}'..='\u{9fff}'
                | '\u{f900}'..='\u{faff}'
                | '\u{ff66}'..='\u{ff9f}' // 半角カタカナ
            )
    })
}

/// ローマ字をカタカナにする。ローマ字として読み切れない部分が残れば None。
fn romaji_to_katakana(romaji: &str) -> Option<String> {
    let mut prepared = String::with_capacity(romaji.len());
    for c in romaji.chars().flat_map(char::to_lowercase) {
        match c {
            // 長音記号を展開する（Kōjō → koujou）
            'ā' | 'â' => prepared.push_str("aa"),
            'ī' | 'î' => prepared.push_str("ii"),
            'ū' | 'û' => prepared.push_str("uu"),
            'ē' | 'ê' => prepared.push_str("ee"),
            'ō' | 'ô' => prepared.push_str("ou"),
            // wana_kana は l を小書きの接頭辞として読む（Lui が ぅい になる）ので R として読む
            'l' => prepared.push('r'),
            _ => prepared.push(c),
        }
    }
    let kana = prepared.to_katakana();
    (!kana.chars().any(|c| c.is_ascii_alphabetic())).then_some(kana)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sort_tag(kana: &str) -> Option<Reading> {
        Some(Reading {
            kana: kana.to_owned(),
            source: ReadingSource::SortTag,
        })
    }

    fn name(kana: &str) -> Option<Reading> {
        Some(Reading {
            kana: kana.to_owned(),
            source: ReadingSource::Name,
        })
    }

    #[test]
    fn kana_sort_tag() {
        assert_eq!(
            reading(
                "キャンディスターにお願い",
                Some("きゃんでぃすたーにおねがい")
            ),
            sort_tag("キャンディスターニオネガイ")
        );
        assert_eq!(
            reading("田村ゆかり", Some("たむら ゆかり")),
            sort_tag("タムラ ユカリ")
        );
    }

    #[test]
    fn romaji_sort_tag() {
        assert_eq!(
            reading("女子力向上宣言！", Some("Joshiryoku Koujou Sengen!")),
            sort_tag("ジョシリョク コウジョウ センゲン！")
        );
        assert_eq!(
            reading("鷹嶺ルイ", Some("Takane Lui")),
            sort_tag("タカネ ルイ")
        );
        assert_eq!(reading("向上", Some("Kōjō")), sort_tag("コウジョウ"));
    }

    #[test]
    fn romaji_sort_tag_needs_japanese_name() {
        // UNISON SQUARE GARDEN の ARTISTSORT をカタカナにしない
        assert_eq!(reading("Rin", Some("Rin")), None);
    }

    #[test]
    fn english_sort_tag_is_rejected() {
        assert_eq!(reading("片羽", Some("Katahane feat. Someone")), None);
        assert_eq!(
            reading("返信願望", Some("Henshin Ganbou (Short ver.)")),
            None
        );
    }

    #[test]
    fn kana_name() {
        assert_eq!(reading("やなぎなぎ", None), name("ヤナギナギ"));
        assert_eq!(reading("ユキトキ", None), name("ユキトキ"));
        assert_eq!(reading("お返事まだカナ", None), None);
        assert_eq!(reading("〈ものがたり〉", None), name("〈モノガタリ〉"));
        assert_eq!(reading("ｶﾀｶﾅ", None), name("カタカナ"));
    }

    #[test]
    fn digits_are_not_symbols() {
        assert_eq!(reading("4U", None), None);
        assert_eq!(reading("3月のパンタシア", None), None);
    }

    #[test]
    fn symbols_only_is_not_a_reading() {
        assert_eq!(reading("!!!", None), None);
    }

    #[test]
    fn falls_back_to_name() {
        assert_eq!(
            reading("やなぎなぎ", Some("Yanagi Nagi (English)")),
            name("ヤナギナギ")
        );
    }
}

//! 読みの規則（docs/schema.md の「日本語の並べ替え」）。
//! 手動設定はここでは扱わない。

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
    Estimated,
}

impl ReadingSource {
    /// DB の `sort_name_source` に入れる値
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SortTag => "sort_tag",
            Self::Name => "name",
            Self::Estimated => "estimated",
        }
    }
}

/// 名前とソート用タグから読みを決める。どちらからも得られなければ、形態素解析で推定する。
pub fn reading(name: &str, sort_tag: Option<&str>) -> Option<Reading> {
    let from_sort_tag = sort_tag.and_then(|sort| {
        let sort: String = sort.nfkc().collect();
        if sort.chars().any(is_kanji) {
            // 名前をそのまま書いたタグは読みではない
            None
        } else if sort.chars().any(is_kana) {
            // かなと混ぜた英字は英語や略語なので、ローマ字として読まずに残す
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
    if is_kana_text(&name) {
        return Some(Reading {
            kana: hiragana_to_katakana(&name),
            source: ReadingSource::Name,
        });
    }
    estimate(&name).map(|kana| Reading {
        kana,
        source: ReadingSource::Estimated,
    })
}

/// 漢字を含む名前を、空白で区切った語ごとに解析して読みをつなぐ。
/// 英字、数字、記号はそのまま残し、読めない漢字が残れば None。
fn estimate(name: &str) -> Option<String> {
    if !name.chars().any(is_kanji) {
        return None;
    }
    let words = name
        .split_whitespace()
        .map(|word| {
            let mut kana = String::new();
            // 英字の並びは解析に渡さない。辞書には NHK電子音楽スタジオ のように英字を含む語もあるため
            for (latin, part) in runs(word, |c| c.is_ascii_alphabetic()) {
                if latin {
                    kana.push_str(part);
                } else {
                    kana.push_str(&read(part)?);
                }
            }
            Some(kana)
        })
        .collect::<Option<Vec<_>>>()?;
    Some(words.join(" "))
}

/// 英字を含まない部分を解析して読む。数字と記号は残す。
fn read(text: &str) -> Option<String> {
    let mut kana = String::new();
    for token in suisei_dict::tokenize(text) {
        let japanese = token.surface.chars().any(|c| is_kana(c) || is_kanji(c));
        if !japanese {
            // 数字や記号だけの語は、辞書に読みがあっても残す
            kana.push_str(&token.surface);
        } else if token.surface.chars().any(|c| c.is_ascii_digit()) {
            // 3月 のように数字と一語になった語は、数字を残して残りを読み直す。
            // 数字の前で切ってから解析すると、第2章 の 章 のように文脈を失って読み違えるため
            for (digits, part) in runs(&token.surface, |c| c.is_ascii_digit()) {
                if digits {
                    kana.push_str(part);
                } else {
                    kana.push_str(&read(part)?);
                }
            }
        } else if let Some(reading) = token.reading {
            kana.push_str(&reading);
        } else if token.surface.chars().any(is_kanji) {
            return None;
        } else {
            kana.push_str(&hiragana_to_katakana(&token.surface));
        }
    }
    Some(kana)
}

/// `pred` に合う文字の並びとそれ以外に分ける。合う並びなら true。
fn runs(text: &str, pred: impl Fn(char) -> bool) -> Vec<(bool, &str)> {
    let mut runs = Vec::new();
    let mut start = 0;
    let mut current = None;
    for (i, c) in text.char_indices() {
        let matched = pred(c);
        if current.is_some_and(|m| m != matched) {
            runs.push((current == Some(true), &text[start..i]));
            start = i;
        }
        current = Some(matched);
    }
    if let Some(matched) = current {
        runs.push((matched, &text[start..]));
    }
    runs
}

fn is_kanji(c: char) -> bool {
    matches!(c,
        '\u{3005}' // 々
        | '\u{3400}'..='\u{4dbf}'
        | '\u{4e00}'..='\u{9fff}'
        | '\u{f900}'..='\u{faff}'
        | '\u{20000}'..='\u{3134f}' // 拡張 B 以降
    )
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
        is_kana(c) || is_kanji(c) || matches!(c, '\u{ff66}'..='\u{ff9f}') // 半角カタカナ
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
            // 「姓, 名」形式の区切り。wana_kana は読点にするが、読みには要らない
            ',' => {}
            // Kuma-san のような区切り。wana_kana は長音記号にする
            '-' => {}
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

    fn estimated(kana: &str) -> Option<Reading> {
        Some(Reading {
            kana: kana.to_owned(),
            source: ReadingSource::Estimated,
        })
    }

    #[test]
    fn estimates_kanji_names() {
        // 辞書を選んだときのベンチマーク
        assert_eq!(reading("米津玄師", None), estimated("ヨネヅケンシ"));
        assert_eq!(reading("富田美憂", None), estimated("トミタミユ"));
        assert_eq!(
            reading("君の知らない物語", None),
            estimated("キミノシラナイモノガタリ")
        );
        // 語ごとに読み、英字はそのまま残す
        assert_eq!(
            reading("歌物語 Special Edition", None),
            estimated("ウタモノガタリ Special Edition")
        );
        // 辞書に読みのある英字も残す
        assert_eq!(
            reading("NHK電子音楽スタジオ", None),
            estimated("NHKデンシオンガクスタジオ")
        );
        assert_eq!(reading("でんぱ組.inc", None), estimated("デンパグミ.inc"));
        // ソート用タグがあれば、そちらを採る
        assert_eq!(
            reading("米津玄師", Some("よねず けんし")),
            sort_tag("ヨネズ ケンシ")
        );
    }

    #[test]
    fn unknown_kanji_is_not_estimated() {
        assert_eq!(reading("ClariS", None), None);
        // 辞書にない漢字が残れば、推定しない
        assert_eq!(reading("𠮷", None), None);
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
    fn mixed_sort_tag_keeps_latin() {
        // かなと混ぜた英字はローマ字として読まない
        assert_eq!(reading("歌手A", Some("かしゅA")), sort_tag("カシュA"));
        assert_eq!(
            reading("永訣のGemini", Some("えいけつのGemini")),
            sort_tag("エイケツノGemini")
        );
        assert_eq!(
            reading(
                "U.N.オーエンは彼女なのか？",
                Some("U.N.おーえんはかのじょなのか？")
            ),
            sort_tag("U.N.オーエンハカノジョナノカ?")
        );
        assert_eq!(
            reading(
                "I LOVE MEでいられるように",
                Some("I LOVE MEでいられるように")
            ),
            sort_tag("I LOVE MEデイラレルヨウニ")
        );
    }

    #[test]
    fn kanji_sort_tag_is_not_a_reading() {
        // 名前をそのまま書いたタグは使わず、推定に回す
        assert_eq!(
            source(reading("A吉スタジオ", Some("A吉スタジオ"))),
            Some(ReadingSource::Estimated)
        );
        assert_eq!(reading("𠮷", Some("𠮷")), None);
    }

    #[test]
    fn hyphen_in_romaji_is_a_separator() {
        assert_eq!(
            reading("はちみつくまさん", Some("Hachimitsu Kuma-san")),
            sort_tag("ハチミツ クマサン")
        );
    }

    #[test]
    fn family_name_first_sort_tag() {
        assert_eq!(
            reading("やなぎなぎ", Some("Yanagi, Nagi")),
            sort_tag("ヤナギ ナギ")
        );
        assert_eq!(
            reading("芹澤優", Some("Serizawa, Yū")),
            sort_tag("セリザワ ユウ")
        );
    }

    #[test]
    fn romaji_sort_tag_needs_japanese_name() {
        // UNISON SQUARE GARDEN の ARTISTSORT をカタカナにしない
        assert_eq!(reading("Rin", Some("Rin")), None);
    }

    fn source(reading: Option<Reading>) -> Option<ReadingSource> {
        reading.map(|r| r.source)
    }

    #[test]
    fn english_sort_tag_is_rejected() {
        // ソート用タグを使わず、推定に回す
        assert_eq!(
            source(reading("片羽", Some("Katahane feat. Someone"))),
            Some(ReadingSource::Estimated)
        );
        assert_eq!(
            source(reading("返信願望", Some("Henshin Ganbou (Short ver.)"))),
            Some(ReadingSource::Estimated)
        );
    }

    #[test]
    fn kana_name() {
        assert_eq!(reading("やなぎなぎ", None), name("ヤナギナギ"));
        assert_eq!(reading("ユキトキ", None), name("ユキトキ"));
        // 漢字を含む名前は、名前をそのまま読みにせず推定に回す
        assert_eq!(
            source(reading("お返事まだカナ", None)),
            Some(ReadingSource::Estimated)
        );
        assert_eq!(reading("〈ものがたり〉", None), name("〈モノガタリ〉"));
        assert_eq!(reading("ｶﾀｶﾅ", None), name("カタカナ"));
    }

    #[test]
    fn digits_are_not_symbols() {
        assert_eq!(reading("4U", None), None);
        assert_eq!(
            source(reading("3月のパンタシア", None)),
            Some(ReadingSource::Estimated)
        );
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

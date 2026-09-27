//! 並べ替えのキーと、アーティスト一覧の見出し（docs/schema.md の「日本語の並べ替え」）。
//!
//! キーは「文字の種類」「比べる文字列」「元の文字列」をつないだもの。バイト列の順に並べればよいので、
//! DB の照合順序に頼らずに `ORDER BY` で使える。

use unicode_normalization::UnicodeNormalization;

/// 比べる文字列と元の文字列の区切り。どの文字よりも小さい
const SEP: char = '\u{1}';

/// 文字の種類。この順に並べる
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Class {
    Symbol,
    Digit,
    Latin,
    Kana,
    /// 読みのない漢字など、五十音の行に入れられないもの
    Other,
}

impl Class {
    fn tag(self) -> char {
        match self {
            Self::Symbol => '0',
            Self::Digit => '1',
            Self::Latin => '2',
            Self::Kana => '3',
            Self::Other => '4',
        }
    }
}

/// 五十音の行。長音記号を母音に置き換えるのにも使う
const ROWS: [(&str, [Option<char>; 5]); 10] = [
    (
        "あ",
        [Some('ア'), Some('イ'), Some('ウ'), Some('エ'), Some('オ')],
    ),
    (
        "か",
        [Some('カ'), Some('キ'), Some('ク'), Some('ケ'), Some('コ')],
    ),
    (
        "さ",
        [Some('サ'), Some('シ'), Some('ス'), Some('セ'), Some('ソ')],
    ),
    (
        "た",
        [Some('タ'), Some('チ'), Some('ツ'), Some('テ'), Some('ト')],
    ),
    (
        "な",
        [Some('ナ'), Some('ニ'), Some('ヌ'), Some('ネ'), Some('ノ')],
    ),
    (
        "は",
        [Some('ハ'), Some('ヒ'), Some('フ'), Some('ヘ'), Some('ホ')],
    ),
    (
        "ま",
        [Some('マ'), Some('ミ'), Some('ム'), Some('メ'), Some('モ')],
    ),
    ("や", [Some('ヤ'), None, Some('ユ'), None, Some('ヨ')]),
    (
        "ら",
        [Some('ラ'), Some('リ'), Some('ル'), Some('レ'), Some('ロ')],
    ),
    ("わ", [Some('ワ'), Some('ヰ'), None, Some('ヱ'), Some('ヲ')]),
];
const VOWELS: [char; 5] = ['ア', 'イ', 'ウ', 'エ', 'オ'];

/// 読みがあれば読みで、なければ名前で並べるキーを作る。
pub fn sort_key(name: &str, reading: Option<&str>) -> String {
    let text: String = reading
        .unwrap_or(name)
        .nfkc()
        .map(hiragana_to_katakana)
        .collect();
    let class = text.chars().next().map_or(Class::Symbol, classify);
    let mut key = String::with_capacity(text.len() * 2 + 2);
    key.push(class.tag());
    key.push_str(&primary(&text));
    key.push(SEP);
    key.push_str(&text);
    key
}

/// キーから、アーティスト一覧の見出しを決める。
/// `#`（記号と数字）、`A`〜`Z`、五十音の行（`あ`〜`わ`）、`他`（それ以外）。キーの順に並べると、見出しもこの順になる。
pub fn index_heading(key: &str) -> String {
    let mut chars = key.chars();
    let class = chars.next();
    let first = chars.next().filter(|c| *c != SEP);
    match (class, first) {
        (Some('2'), Some(c)) => c.to_ascii_uppercase().to_string(),
        (Some('3'), Some(c)) => kana_row(c).unwrap_or("#").to_owned(),
        (Some('4'), _) => "他".to_owned(),
        _ => "#".to_owned(),
    }
}

fn classify(c: char) -> Class {
    let base = base_char(c);
    if c.is_ascii_digit() {
        Class::Digit
    } else if base.is_ascii_alphabetic() {
        Class::Latin
    } else if is_katakana(c) || c == 'ー' {
        Class::Kana
    } else if c.is_alphanumeric() {
        Class::Other
    } else {
        Class::Symbol
    }
}

/// 一次の比較に使う文字列。JIS X 4061 に倣い、記号と空白を除き、
/// 濁点、半濁点、アクセントを外し、小書きを大きくし、長音記号を直前の母音に置き換え、英字は小文字にする。
fn primary(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut vowel = None;
    for c in text.chars() {
        if c == 'ー' {
            if let Some(v) = vowel {
                out.push(v);
            }
            continue;
        }
        if !c.is_alphanumeric() {
            continue;
        }
        let base = enlarge(base_char(c));
        vowel = vowel_of(base);
        out.extend(base.to_lowercase());
    }
    out
}

/// 分解して、濁点やアクセントを外した文字。
fn base_char(c: char) -> char {
    c.nfd().next().unwrap_or(c)
}

fn hiragana_to_katakana(c: char) -> char {
    match c {
        '\u{3041}'..='\u{3096}' => char::from_u32(c as u32 + 0x60).unwrap_or(c),
        _ => c,
    }
}

fn is_katakana(c: char) -> bool {
    matches!(c, '\u{30a1}'..='\u{30fa}')
}

fn enlarge(c: char) -> char {
    match c {
        'ァ' => 'ア',
        'ィ' => 'イ',
        'ゥ' => 'ウ',
        'ェ' => 'エ',
        'ォ' => 'オ',
        'ッ' => 'ツ',
        'ャ' => 'ヤ',
        'ュ' => 'ユ',
        'ョ' => 'ヨ',
        'ヮ' => 'ワ',
        'ヵ' => 'カ',
        'ヶ' => 'ケ',
        _ => c,
    }
}

fn vowel_of(c: char) -> Option<char> {
    ROWS.iter()
        .find_map(|(_, row)| row.iter().position(|k| *k == Some(c)).map(|i| VOWELS[i]))
}

fn kana_row(c: char) -> Option<&'static str> {
    if c == 'ン' {
        return Some("わ");
    }
    ROWS.iter()
        .find(|(_, row)| row.contains(&Some(c)))
        .map(|(heading, _)| *heading)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sorted<'a>(names: &[(&'a str, Option<&'a str>)]) -> Vec<&'a str> {
        let mut keyed: Vec<_> = names.iter().map(|(n, r)| (sort_key(n, *r), *n)).collect();
        keyed.sort();
        keyed.into_iter().map(|(_, n)| n).collect()
    }

    fn heading(name: &str, reading: Option<&str>) -> String {
        index_heading(&sort_key(name, reading))
    }

    #[test]
    fn classes_in_order() {
        assert_eq!(
            sorted(&[
                ("魚図鑑", None),
                ("やなぎなぎ", Some("ヤナギナギ")),
                ("ClariS", None),
                ("4U", None),
                ("[Alexandros]", None),
            ]),
            ["[Alexandros]", "4U", "ClariS", "やなぎなぎ", "魚図鑑"]
        );
    }

    #[test]
    fn latin_ignores_case_and_accents() {
        assert_eq!(
            sorted(&[("b", None), ("Ámbar", None), ("apple", None)]),
            ["Ámbar", "apple", "b"]
        );
        assert_eq!(heading("ámbar", None), "A");
    }

    #[test]
    fn kana_follows_jis_x_4061() {
        // 濁点と小書きは一次では区別しない。清音が先
        assert_eq!(
            sorted(&[("ガ", None), ("カキ", None), ("カ", None)]),
            ["カ", "ガ", "カキ"]
        );
        // 長音記号は直前の母音として比べる（カー は カア と同じ位置）
        assert_eq!(
            sorted(&[("カイ", None), ("カー", None), ("カア", None)]),
            ["カア", "カー", "カイ"]
        );
        // 記号と空白は一次では無視する
        assert_eq!(
            sorted(&[("タムラ ユカリ", None), ("タムラカ", None)]),
            ["タムラカ", "タムラ ユカリ"]
        );
    }

    #[test]
    fn reading_is_preferred() {
        assert_eq!(
            sorted(&[("米津玄師", Some("ヨネズ ケンシ")), ("アイ", None)]),
            ["アイ", "米津玄師"]
        );
    }

    #[test]
    fn headings() {
        assert_eq!(heading("[Alexandros]", None), "#");
        assert_eq!(heading("4U", None), "#");
        assert_eq!(heading("ClariS", None), "C");
        assert_eq!(heading("やなぎなぎ", Some("ヤナギナギ")), "や");
        assert_eq!(heading("ヴィヴィッド", None), "あ");
        assert_eq!(heading("ンダホ", None), "わ");
        assert_eq!(heading("お返事まだカナ", None), "あ");
        assert_eq!(heading("魚図鑑", None), "他");
        assert_eq!(heading("", None), "#");
    }
}

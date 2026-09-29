//! `キャラクター(CV:声優)` の形の名前を分ける（docs/schema.md の「アーティスト」）。

/// 分けた名前をユニット、キャラクター、声優の順に返す。形が合わなければ None。
pub(super) fn split(name: &str) -> Option<Vec<String>> {
    let name = name.trim();
    unit_with_brackets(name).or_else(|| members(name, true))
}

/// `ユニット [A(CV:x)、B(CV:y)]` の形。
fn unit_with_brackets(name: &str) -> Option<Vec<String>> {
    let close = name.chars().next_back()?;
    let open = match close {
        ']' | '］' => '[',
        '》' => '《',
        ')' | '）' => '(',
        '】' => '【',
        _ => return None,
    };
    let start = matching_open(name, open)?;
    let unit = name[..start].trim();
    let inner = &name[start + open_len(name, start)..name.len() - close.len_utf8()];
    if unit.is_empty() {
        return None;
    }
    let mut names = vec![unit.to_owned()];
    names.extend(members(inner, false)?);
    Some(names)
}

/// 最後の閉じ括弧に対応する開き括弧の位置。
fn matching_open(name: &str, open: char) -> Option<usize> {
    let is_open = |c: char| c == open || (open == '(' && c == '（') || (open == '[' && c == '［');
    let is_close = |c: char| match open {
        '(' => c == ')' || c == '）',
        '[' => c == ']' || c == '］',
        '《' => c == '》',
        _ => c == '】',
    };
    let mut depth = 0usize;
    for (i, c) in name.char_indices().rev() {
        if is_close(c) {
            depth += 1;
        } else if is_open(c) {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(i);
            }
        }
    }
    None
}

fn open_len(name: &str, start: usize) -> usize {
    name[start..].chars().next().map_or(0, char::len_utf8)
}

/// 断片の頭に来る区切り。キャラクター名の途中の `・` などは区切らない。
const SEPARATORS: &[char] = &['、', ',', '，', '&', '＆', '・', '/', '／', '×'];

/// `A(CV:x)、B(CV:y)` の列挙。`allow_unit` なら、先頭の `ユニット: A(CV:x)` と `ユニット/A(CV:x)` も受け付ける。
fn members(list: &str, allow_unit: bool) -> Option<Vec<String>> {
    let mut names = Vec::new();
    let mut rest = list;
    let mut first = true;
    while let Some(group) = cv_group(rest) {
        let mut character = rest[..group.start]
            .trim_start_matches(|c: char| c.is_whitespace() || (!first && SEPARATORS.contains(&c)));
        if first
            && allow_unit
            && let Some((unit, member)) = character.split_once([':', '：', '/', '／'])
            && !unit.trim().is_empty()
        {
            names.push(unit.trim().to_owned());
            character = member;
        }
        let character = character.trim();
        if !valid_character(character) {
            return None;
        }
        names.push(character.to_owned());
        names.push(group.voice.to_owned());
        rest = &rest[group.end..];
        first = false;
    }
    if first {
        return None;
    }
    // 列挙の後ろには、省略を表す「他」しか置けない
    match rest.trim() {
        "" | "他" | "ほか" => Some(names),
        _ => None,
    }
}

fn valid_character(character: &str) -> bool {
    let lower = character.to_lowercase();
    !character.is_empty()
        && !character.contains(['(', ')', '（', '）'])
        && !lower.contains("feat.")
        && !lower.contains(" ft.")
}

/// `(CV:声優)` の位置と声優の名前。
struct CvGroup<'a> {
    start: usize,
    end: usize,
    voice: &'a str,
}

/// 最初の `(CV:声優)` を探す。
fn cv_group(text: &str) -> Option<CvGroup<'_>> {
    for (start, c) in text.char_indices() {
        if c != '(' && c != '（' {
            continue;
        }
        let inner = &text[start + c.len_utf8()..];
        let close = inner.find([')', '）'])?;
        let Some(voice) = voice(&inner[..close]) else {
            continue;
        };
        let close_len = inner[close..].chars().next().map_or(1, char::len_utf8);
        return Some(CvGroup {
            start,
            end: start + c.len_utf8() + close + close_len,
            voice,
        });
    }
    None
}

/// 括弧の中身が `CV:声優` なら声優の名前。声優がまとめて書かれていれば None。
fn voice(inner: &str) -> Option<&str> {
    let inner = inner.trim_start();
    let mut chars = inner.char_indices();
    let (_, c) = chars.next()?;
    let (_, v) = chars.next()?;
    if !matches!(c, 'C' | 'c' | 'Ｃ' | 'ｃ') || !matches!(v, 'V' | 'v' | 'Ｖ' | 'ｖ') {
        return None;
    }
    let after = &inner[c.len_utf8() + v.len_utf8()..];
    // `(CVS)` のような略語と区別するため、CV の直後に英数字が続くものは除く
    if after
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphanumeric())
    {
        return None;
    }
    let voice = after
        .trim_start()
        .trim_start_matches(['.', '．', ':', '：'])
        .trim();
    if voice.is_empty() || voice.contains(['、', ',', '，', '&', '＆', '/', '／', '×']) {
        return None;
    }
    Some(voice)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(name: &str) -> Option<Vec<String>> {
        split(name)
    }

    fn some(names: &[&str]) -> Option<Vec<String>> {
        Some(names.iter().map(|&n| n.to_owned()).collect())
    }

    #[test]
    fn one_character() {
        for name in [
            "後藤ひとり(CV:青山吉能)",
            "後藤ひとり(CV.青山吉能)",
            "後藤ひとり (CV. 青山吉能)",
            "後藤ひとり（CV：青山吉能）",
            "後藤ひとり(cv.青山吉能)",
            "後藤ひとり(Cv.青山吉能)",
            "後藤ひとり( CV.青山吉能)",
            "後藤ひとり(CV 青山吉能)",
            "後藤ひとり(ＣＶ．青山吉能)",
        ] {
            assert_eq!(names(name), some(&["後藤ひとり", "青山吉能"]), "{name}");
        }
        assert_eq!(
            names("Kiryu Moeka (CV. Goto Saori)"),
            some(&["Kiryu Moeka", "Goto Saori"])
        );
    }

    #[test]
    fn listed_characters() {
        assert_eq!(
            names("鈴仙・優曇華院・イナバ(cv.さくらみこ), 因幡てゐ(cv.兎田ぺこら)"),
            some(&[
                "鈴仙・優曇華院・イナバ",
                "さくらみこ",
                "因幡てゐ",
                "兎田ぺこら"
            ])
        );
        assert_eq!(
            names("ひーなー(CV.鬼頭明里)&かーなー(CV.ファイルーズあい)"),
            some(&["ひーなー", "鬼頭明里", "かーなー", "ファイルーズあい"])
        );
        assert_eq!(
            names("栗山未来(CV.種田梨沙)×名瀬美月(CV.茅原実里)"),
            some(&["栗山未来", "種田梨沙", "名瀬美月", "茅原実里"])
        );
        assert_eq!(
            names("マヤ (CV:水瀬いのり) エリカ (CV: 伊波杏樹)"),
            some(&["マヤ", "水瀬いのり", "エリカ", "伊波杏樹"])
        );
        assert_eq!(
            names("泉こなた( CV.平野綾), 柊かがみ( CV.加藤英美里) 他"),
            some(&["泉こなた", "平野綾", "柊かがみ", "加藤英美里"])
        );
    }

    #[test]
    fn unit_and_members() {
        let expected = some(&[
            "あんこうチーム",
            "西住みほ",
            "渕上舞",
            "武部沙織",
            "茅野愛衣",
        ]);
        for name in [
            "あんこうチーム [西住みほ(CV.渕上舞)、武部沙織(CV.茅野愛衣)]",
            "あんこうチーム《西住みほ(CV:渕上舞)、武部沙織(CV:茅野愛衣)》",
            "あんこうチーム(西住みほ(CV.渕上舞)、武部沙織(CV.茅野愛衣))",
            "あんこうチーム: 西住みほ(CV:渕上舞)、武部沙織(CV:茅野愛衣)",
            "あんこうチーム/西住みほ(CV:渕上舞)、武部沙織(CV:茅野愛衣)",
        ] {
            assert_eq!(names(name), expected, "{name}");
        }
    }

    #[test]
    fn other_forms_are_kept() {
        for name in [
            "ClariS",
            "[Alexandros]",
            "〈物語〉シリーズ",
            "C.V. Jørgensen",
            "CV Massage",
            "アンジェラ(Vo.Alisa)",
            "(CV:声優)",
            "Artist (CVS Remix)",
            "IOSYS feat. チルノ（CV.ファイルーズあい）",
            "桜高軽音部 [平沢唯・秋山澪(CV:豊崎愛生、日笠陽子)]",
            "立花響×風鳴翼(CV:悠木碧×水樹奈々)",
            "山田一郎&山田二郎(CV.木村 昴&野津山幸宏)",
            "A/B(CV.野田順子/福島潤)",
            "放課後スイーツ部《アイリ(CV:水森ちこ)、ヨシミ)》",
            "後藤ひとり(CV:青山吉能) with 結束バンド",
        ] {
            assert_eq!(names(name), None, "{name}");
        }
    }
}

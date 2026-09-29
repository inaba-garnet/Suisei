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

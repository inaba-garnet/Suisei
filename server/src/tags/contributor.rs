//! 作曲、作詞、編曲の名前を一人ずつに分ける（docs/schema.md の「作曲、作詞、編曲」）。

/// 共同で作った曲の名前を区切る文字。`・` と空白は名前の中にも現れるので区切らない。
const SEPARATORS: &[char] = &['、', ',', '，', '/', '／', ';', '&', '＆'];

/// タグの値から一人ずつの名前を返す。括弧の中の区切りは区切らず、末尾の括弧（所属など）は外す。
pub(super) fn names(values: &[String]) -> Vec<String> {
    values
        .iter()
        .flat_map(|value| split(value))
        .filter_map(|name| {
            let name = strip_suffix(name.trim()).trim();
            (!name.is_empty()).then(|| name.to_owned())
        })
        .collect()
}

/// 括弧の外にある区切りで分ける。
fn split(value: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0usize;
    let mut start = 0;
    for (i, c) in value.char_indices() {
        match c {
            '(' | '（' => depth += 1,
            ')' | '）' => depth = depth.saturating_sub(1),
            _ if depth == 0 && SEPARATORS.contains(&c) => {
                parts.push(&value[start..i]);
                start = i + c.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(&value[start..]);
    parts
}

/// 末尾の括弧を外す。括弧の前に名前が残らないときは外さない。
fn strip_suffix(name: &str) -> &str {
    let Some(close) = name.chars().next_back() else {
        return name;
    };
    if close != ')' && close != '）' {
        return name;
    }
    let mut depth = 0usize;
    for (i, c) in name.char_indices().rev() {
        match c {
            ')' | '）' => depth += 1,
            '(' | '（' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    let before = name[..i].trim_end();
                    return if before.is_empty() { name } else { before };
                }
            }
            _ => {}
        }
    }
    name
}

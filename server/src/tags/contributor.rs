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
#[cfg(test)]
mod tests {
    use super::*;

    fn run(values: &[&str]) -> Vec<String> {
        names(&values.iter().map(|v| (*v).to_owned()).collect::<Vec<_>>())
    }

    #[test]
    fn joint_credits_are_split() {
        assert_eq!(
            run(&[
                "荒幡亮平、堀江晶太、rui（fade）、⽑蟹（LIVE LAB.）、ハヤシケイ（LIVE LAB.）、神崎エルザ"
            ]),
            [
                "荒幡亮平",
                "堀江晶太",
                "rui",
                "⽑蟹",
                "ハヤシケイ",
                "神崎エルザ"
            ]
        );
        assert_eq!(run(&["A, B & C"]), ["A", "B", "C"]);
        assert_eq!(run(&["A／B"]), ["A", "B"]);
        assert_eq!(run(&["A; B"]), ["A", "B"]);
    }

    #[test]
    fn multiple_values_are_kept_apart() {
        assert_eq!(run(&["eba", "睦月周平"]), ["eba", "睦月周平"]);
    }

    #[test]
    fn middle_dots_and_spaces_are_not_separators() {
        assert_eq!(run(&["ジョン・レノン"]), ["ジョン・レノン"]);
        assert_eq!(run(&["HIDEO NEKOTA"]), ["HIDEO NEKOTA"]);
    }

    #[test]
    fn affiliations_are_removed() {
        assert_eq!(run(&["藤永龍太郎(Elements Garden)"]), ["藤永龍太郎"]);
        assert_eq!(run(&["本多友紀 (Arte Refact)"]), ["本多友紀"]);
        // 括弧の中の区切りでは分けない
        assert_eq!(run(&["A（X、Y）、B"]), ["A", "B"]);
        // 名前が括弧だけなら残す
        assert_eq!(run(&["(仮)"]), ["(仮)"]);
    }

    #[test]
    fn empty_parts_are_dropped() {
        assert_eq!(run(&["A、、B、"]), ["A", "B"]);
        assert!(run(&[" "]).is_empty());
    }
}

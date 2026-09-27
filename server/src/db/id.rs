/// ID の種類と接頭辞。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdKind {
    Artist,
    Album,
    Track,
    File,
    Playlist,
}

impl IdKind {
    fn prefix(self) -> &'static str {
        match self {
            Self::Artist => "ar",
            Self::Album => "al",
            Self::Track => "tr",
            Self::File => "fi",
            Self::Playlist => "pl",
        }
    }
}

/// 接頭辞と 16 進 8 文字の乱数をつなげた ID を作る。
/// 重複はありうるので、呼び出し側が一意制約の違反を見て採り直す。
pub fn new_id(kind: IdKind) -> String {
    let mut bytes = [0u8; 4];
    getrandom::fill(&mut bytes).expect("OS の乱数源が使えない");
    format!("{}-{}", kind.prefix(), hex::encode(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format() {
        let id = new_id(IdKind::Track);
        let (prefix, hex) = id.split_once('-').unwrap();
        assert_eq!(prefix, "tr");
        assert_eq!(hex.len(), 8);
        assert!(
            hex.bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        );
    }
}

//! 読みの推定に使う形態素解析（docs/server.md の「形態素解析」）。
//! 辞書は build.rs で作ってバイナリに埋め込む。

use std::borrow::Cow;
use std::sync::LazyLock;

use lindera::mode::Mode;
use lindera::segmenter::Segmenter;

lindera_dictionary::embedded_dictionary!("/suisei", EmbeddedLoader);

static SEGMENTER: LazyLock<Segmenter> = LazyLock::new(|| {
    Segmenter::new(
        Mode::Normal,
        load().expect("埋め込んだ辞書を読めない"),
        None,
    )
});

/// 解析した語と、辞書にあればその読み（カタカナ）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub surface: String,
    pub reading: Option<String>,
}

/// 語に分けて、それぞれの読みを返す。
pub fn tokenize(text: &str) -> Vec<Token> {
    let Ok(mut tokens) = SEGMENTER.segment(Cow::Borrowed(text)) else {
        return Vec::new();
    };
    tokens
        .iter_mut()
        .map(|token| Token {
            surface: token.surface.to_string(),
            reading: token
                .get("reading")
                .filter(|r| !r.is_empty() && *r != "*")
                .map(str::to_owned),
        })
        .collect()
}

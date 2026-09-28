//! `stream` のトランスコード（docs/schema.md の「配信ファイル」、docs/server.md の「トランスコード」）。

use std::path::Path;
use std::process::Stdio;

use axum::body::Body;
use axum::http::header::{ACCEPT_RANGES, CONTENT_LENGTH, CONTENT_TYPE};
use axum::http::{HeaderValue, Method, StatusCode};
use axum::response::Response;
use futures_util::StreamExt;
use tokio::process::Command;
use tokio_util::io::ReaderStream;

/// ビットレートの上限（kbps）。`maxBitRate` のない変換もこの値にする。
const MAX_BIT_RATE: u32 = 320;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Codec {
    Mp3,
    Opus,
}

impl Codec {
    fn from_format(format: &str) -> Option<Self> {
        match format.to_ascii_lowercase().as_str() {
            "mp3" => Some(Self::Mp3),
            "opus" => Some(Self::Opus),
            _ => None,
        }
    }

    /// 変換しなくてよい元のファイルの拡張子。
    fn suffix(self) -> &'static str {
        match self {
            Self::Mp3 => "mp3",
            Self::Opus => "opus",
        }
    }

    fn content_type(self) -> &'static str {
        match self {
            Self::Mp3 => "audio/mpeg",
            Self::Opus => "audio/ogg",
        }
    }

    /// ffmpeg のエンコーダと出力の形式。
    fn ffmpeg_args(self) -> [&'static str; 4] {
        match self {
            Self::Mp3 => ["-c:a", "libmp3lame", "-f", "mp3"],
            Self::Opus => ["-c:a", "libopus", "-f", "ogg"],
        }
    }
}

/// 配信するファイルの性質。
#[derive(Debug, Clone)]
pub struct Source<'a> {
    pub suffix: &'a str,
    /// kbps
    pub bit_rate: Option<i64>,
    pub lossless: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Plan {
    pub codec: Codec,
    /// kbps
    pub bit_rate: u32,
}

/// 変換するかどうかと、変換の形式とビットレートを決める。変換しないなら None。
pub fn plan(format: Option<&str>, max_bit_rate: Option<u32>, source: &Source) -> Option<Plan> {
    if format.is_some_and(|f| f.eq_ignore_ascii_case("raw")) {
        return None;
    }
    let max_bit_rate = max_bit_rate.filter(|&b| b > 0);
    let requested = format.and_then(Codec::from_format);
    let codec = match (requested, max_bit_rate) {
        (Some(codec), _) => codec,
        // 形式の指定がなく上限だけあれば、どの端末でも再生できる MP3 にする
        (None, Some(_)) => Codec::Mp3,
        (None, None) => return None,
    };
    let source_rate = source.bit_rate.and_then(|b| u32::try_from(b).ok());
    // 形式の指定がなく、元のビットレートが上限以下なら、変換する理由がない
    if requested.is_none()
        && let (Some(max), Some(rate)) = (max_bit_rate, source_rate)
        && rate <= max
    {
        return None;
    }
    let mut bit_rate = max_bit_rate.unwrap_or(MAX_BIT_RATE).min(MAX_BIT_RATE);
    // 非可逆圧縮の元より高いビットレートにしても、音質は上がらない
    if !source.lossless
        && let Some(rate) = source_rate
    {
        bit_rate = bit_rate.min(rate);
    }
    if source.suffix.eq_ignore_ascii_case(codec.suffix())
        && source_rate.is_some_and(|rate| rate <= bit_rate)
    {
        return None;
    }
    Some(Plan { codec, bit_rate })
}

/// ffmpeg で変換しながら返す。ffmpeg を起動できなければ None で、呼び出し側は元のファイルを返す。
/// `offset` は開始位置（秒）、`estimate` は見積もった `Content-Length` を付けるための曲の長さ（ミリ秒）。
pub fn stream(
    ffmpeg: &Path,
    path: &Path,
    plan: Plan,
    method: &Method,
    offset: Option<f64>,
    estimate: Option<i64>,
) -> Option<Response> {
    let mut builder = Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, plan.codec.content_type())
        // 変換した音声は Range で途中から読めない。途中からは timeOffset で求めてもらう
        .header(ACCEPT_RANGES, HeaderValue::from_static("none"));
    if let Some(duration_ms) = estimate {
        let offset_ms = offset.map_or(0, |o| (o * 1000.0) as i64);
        let remaining_ms = u64::try_from(duration_ms - offset_ms).unwrap_or(0);
        let length = remaining_ms * u64::from(plan.bit_rate) / 8;
        builder = builder.header(CONTENT_LENGTH, length);
    }
    if method == Method::HEAD {
        return builder.body(Body::empty()).ok();
    }

    let mut command = Command::new(ffmpeg);
    command.args(["-nostdin", "-hide_banner", "-loglevel", "error"]);
    if let Some(offset) = offset {
        command.arg("-ss").arg(format!("{offset:.3}"));
    }
    command
        .arg("-i")
        .arg(path)
        .args(["-map", "0:a:0", "-vn", "-map_metadata", "-1"])
        .args(plan.codec.ffmpeg_args())
        .arg("-b:a")
        .arg(format!("{}k", plan.bit_rate))
        .arg("pipe:1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        // ffmpeg のエラーはサーバーのログに流す
        .stderr(Stdio::inherit())
        // クライアントが切断して応答を捨てたら、ffmpeg も止める
        .kill_on_drop(true);
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(err) => {
            tracing::warn!(ffmpeg = %ffmpeg.display(), error = %err, "cannot start ffmpeg");
            return None;
        }
    };
    let stdout = child.stdout.take()?;
    // 応答の本文が読み終わるか捨てられるまで、子プロセスを持っておく
    let body = ReaderStream::new(stdout).map(move |chunk| {
        let _ = &child;
        chunk
    });
    builder.body(Body::from_stream(body)).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const FLAC: Source = Source {
        suffix: "flac",
        bit_rate: Some(900),
        lossless: true,
    };
    const MP3_128: Source = Source {
        suffix: "mp3",
        bit_rate: Some(128),
        lossless: false,
    };

    fn planned(format: Option<&str>, max: Option<u32>, source: &Source) -> Option<(Codec, u32)> {
        plan(format, max, source).map(|p| (p.codec, p.bit_rate))
    }

    #[test]
    fn no_arguments_mean_original() {
        assert_eq!(planned(None, None, &FLAC), None);
        assert_eq!(planned(Some("raw"), Some(128), &FLAC), None);
        // 知らない形式だけなら変換しない
        assert_eq!(planned(Some("aac"), None, &FLAC), None);
    }

    #[test]
    fn format_without_limit_uses_320() {
        assert_eq!(planned(Some("mp3"), None, &FLAC), Some((Codec::Mp3, 320)));
        assert_eq!(planned(Some("opus"), None, &FLAC), Some((Codec::Opus, 320)));
        assert_eq!(
            planned(Some("mp3"), Some(0), &FLAC),
            Some((Codec::Mp3, 320))
        );
    }

    #[test]
    fn limit_without_format_uses_mp3() {
        assert_eq!(planned(None, Some(128), &FLAC), Some((Codec::Mp3, 128)));
        assert_eq!(
            planned(Some("aac"), Some(128), &FLAC),
            Some((Codec::Mp3, 128))
        );
        // 上限は 320 まで
        assert_eq!(planned(None, Some(500), &FLAC), Some((Codec::Mp3, 320)));
        // 元が上限以下なら変換しない
        assert_eq!(planned(None, Some(1000), &FLAC), None);
        assert_eq!(planned(None, Some(192), &MP3_128), None);
    }

    #[test]
    fn lossy_source_is_not_raised() {
        let m4a = Source {
            suffix: "m4a",
            bit_rate: Some(256),
            lossless: false,
        };
        assert_eq!(planned(Some("mp3"), None, &m4a), Some((Codec::Mp3, 256)));
        // 同じ形式で上限以下なら、元のまま
        assert_eq!(planned(Some("mp3"), None, &MP3_128), None);
        assert_eq!(
            planned(Some("mp3"), Some(96), &MP3_128),
            Some((Codec::Mp3, 96))
        );
    }
}

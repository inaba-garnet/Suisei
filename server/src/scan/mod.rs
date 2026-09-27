//! 音楽フォルダを走査し、ライブラリを組み立て直す（docs/schema.md の「スキャン」）。

mod build;

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use walkdir::WalkDir;

use crate::db::Pool;
use crate::db::library::{self, FileRow, Snapshot};
use crate::tags::{self, RawTags};

use build::Scanned;

/// 読む拡張子。lofty が読める音声に限る
const AUDIO_EXTENSIONS: &[&str] = &[
    "aac", "aif", "aiff", "ape", "flac", "m4a", "mp3", "mpc", "oga", "ogg", "opus", "wav", "wv",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// サイズと mtime が前回と同じファイルは読み直さない
    Quick,
    /// すべてのファイルを読み直す
    Full,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Summary {
    pub files: usize,
    /// タグを読んだファイル
    pub read: usize,
    /// 読めなかったファイル
    pub failed: usize,
    pub tracks: usize,
    pub albums: usize,
    pub artists: usize,
}

#[derive(Debug)]
pub enum Error {
    /// 音楽フォルダそのものが読めない
    Folder(std::io::Error),
    /// 音声が一つも見つからないのに、DB にはファイルがある。NAS のマウントが外れたときに全曲を消さないため
    Empty,
    Db(sqlx::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Folder(err) => write!(f, "音楽フォルダを読めない: {err}"),
            Self::Empty => write!(
                f,
                "音楽フォルダに音声がないので、ライブラリを消さずに中断した"
            ),
            Self::Db(err) => write!(f, "DB のエラー: {err}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<sqlx::Error> for Error {
    fn from(err: sqlx::Error) -> Self {
        Self::Db(err)
    }
}

pub async fn run(pool: &Pool, music_dir: &Path, mode: Mode) -> Result<Summary, Error> {
    let snapshot = library::snapshot(pool).await?;
    let music_dir = music_dir.to_owned();
    let (library, summary) = tokio::task::spawn_blocking(move || {
        let (files, mut summary) = collect(&music_dir, &snapshot, mode)?;
        if files.is_empty() && !snapshot.files.is_empty() {
            return Err(Error::Empty);
        }
        let library = build::build(
            files,
            &snapshot,
            &music_dir.to_string_lossy(),
            unix_millis(SystemTime::now()),
        );
        summary.files = library.files.len();
        summary.tracks = library.tracks.len();
        summary.albums = library.albums.len();
        summary.artists = library.artists.len();
        Ok((library, summary))
    })
    .await
    .expect("スキャンのタスクが panic した")?;
    library::replace(pool, &library).await?;
    Ok(summary)
}

/// 音楽フォルダを走査し、変わったファイルだけタグを読む。
fn collect(
    music_dir: &Path,
    snapshot: &Snapshot,
    mode: Mode,
) -> Result<(Vec<Scanned>, Summary), Error> {
    std::fs::read_dir(music_dir).map_err(Error::Folder)?;
    let stored: HashMap<&str, &FileRow> = snapshot
        .files
        .iter()
        .map(|f| (f.path.as_str(), f))
        .collect();
    let mut summary = Summary::default();
    let mut files = Vec::new();
    let mut seen = std::collections::HashSet::new();
    // 読めなかったディレクトリ。その下にあった行は、消さずに残す
    let mut unreadable_dirs: Vec<PathBuf> = Vec::new();

    let walker = WalkDir::new(music_dir)
        .into_iter()
        .filter_entry(|e| e.depth() == 0 || !is_hidden(e.file_name()));
    for entry in walker {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                tracing::warn!(error = %err, "走査できない");
                if let Some(path) = err.path().and_then(|p| p.strip_prefix(music_dir).ok()) {
                    unreadable_dirs.push(path.to_owned());
                }
                continue;
            }
        };
        if !entry.file_type().is_file() || !is_audio(entry.path()) {
            continue;
        }
        let Some(rel) = relative(music_dir, entry.path()) else {
            tracing::warn!(path = %entry.path().display(), "UTF-8 でないパスは扱わない");
            continue;
        };
        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(err) => {
                tracing::warn!(path = rel, error = %err, "ファイルの情報を読めない");
                if let Some(row) = stored.get(rel.as_str()) {
                    files.push(unchanged(row));
                    seen.insert(rel);
                }
                continue;
            }
        };
        let size = i64::try_from(metadata.len()).unwrap_or(i64::MAX);
        let mtime = metadata.modified().map(unix_millis).unwrap_or(0);
        let previous = stored.get(rel.as_str()).copied();
        seen.insert(rel.clone());

        if mode == Mode::Quick
            && let Some(row) = previous.filter(|r| r.size == size && r.mtime == mtime)
            && let Ok(scanned) = parse_stored(row)
        {
            files.push(scanned);
            continue;
        }
        summary.read += 1;
        match tags::read(entry.path()) {
            Ok((tags, props)) => files.push(Scanned {
                row: FileRow {
                    id: previous.map(|r| r.id.clone()).unwrap_or_default(),
                    track_id: previous.map(|r| r.track_id.clone()).unwrap_or_default(),
                    path: rel,
                    size,
                    mtime,
                    suffix: props.suffix,
                    content_type: props.content_type.to_owned(),
                    duration_ms: i64::try_from(props.duration_ms).unwrap_or(i64::MAX),
                    bit_rate: props.bit_rate.map(i64::from),
                    sample_rate: props.sample_rate.map(i64::from),
                    channels: props.channels.map(i64::from),
                    bit_depth: props.bit_depth.map(i64::from),
                    lossless: props.lossless,
                    tags: String::new(),
                },
                tags,
            }),
            Err(err) => {
                summary.failed += 1;
                tracing::warn!(path = rel, error = %err, "タグを読めない");
                // 一時的に読めないだけかもしれないので、前回の行を残す
                if let Some(row) = previous {
                    files.push(unchanged(row));
                }
            }
        }
    }

    for row in &snapshot.files {
        let under_unreadable = unreadable_dirs
            .iter()
            .any(|dir| Path::new(&row.path).starts_with(dir));
        if under_unreadable && !seen.contains(&row.path) {
            files.push(unchanged(row));
        }
    }
    Ok((files, summary))
}

fn parse_stored(row: &FileRow) -> Result<Scanned, serde_json::Error> {
    Ok(Scanned {
        tags: serde_json::from_str::<RawTags>(&row.tags)?,
        row: row.clone(),
    })
}

/// 前回の行をそのまま使う。保存したタグが壊れていれば、タグなしとして扱う
fn unchanged(row: &FileRow) -> Scanned {
    parse_stored(row).unwrap_or_else(|_| Scanned {
        row: row.clone(),
        tags: RawTags::default(),
    })
}

fn is_hidden(name: &std::ffi::OsStr) -> bool {
    name.as_encoded_bytes().starts_with(b".")
}

fn is_audio(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| AUDIO_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
}

/// 音楽フォルダからの相対パス。区切りは `/` にそろえる。
fn relative(root: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(root).ok()?;
    let parts: Option<Vec<&str>> = rel.components().map(|c| c.as_os_str().to_str()).collect();
    Some(parts?.join("/"))
}

fn unix_millis(time: SystemTime) -> i64 {
    time.duration_since(UNIX_EPOCH)
        .map(|d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

//! カバーアートの縮小とキャッシュ（docs/schema.md の「カバーアート」）。

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::UNIX_EPOCH;

use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::{DynamicImage, ImageReader, Rgb, RgbImage};
use tokio::sync::Semaphore;

use super::media::{embedded_picture, is_image};

/// 縮小した画像の JPEG の品質。
const JPEG_QUALITY: u8 = 85;

/// 同時に縮小する数。一斉に求められても、メモリと CPU が跳ね上がらないよう CPU の数までにする。
static RESIZING: LazyLock<Semaphore> =
    LazyLock::new(|| Semaphore::new(std::thread::available_parallelism().map_or(1, |n| n.get())));

/// `size` のないときの上限。`size` を付けないクライアントにも、数 MB の元画像を送らないため。
pub const DEFAULT_SIZE: u32 = 1024;

/// キャッシュに置くファイルの拡張子。縮小した JPEG のほか、縮小で容量が増えるときは元の画像を置く。
const CACHED_EXTENSIONS: [&str; 4] = ["jpg", "png", "webp", "gif"];

/// 縮小した画像のパスを返す。元の画像が `size` 以下のときと、縮小できなかったときは None で、
/// 呼び出し側は元の画像を返す。
pub async fn resized(
    cache_dir: &Path,
    album_id: &str,
    source: &Path,
    size: u32,
) -> Option<PathBuf> {
    let dir = cache_dir.join("cover");
    let mtime = std::fs::metadata(source)
        .and_then(|m| m.modified())
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_millis();
    // 元のファイルの更新日時をキーに含め、画像を差し替えたら作り直す
    let prefix = format!("{album_id}-{size}-");
    let stem = format!("{prefix}{mtime}");
    if let Some(cached) = find_cached(&dir, &stem) {
        return Some(cached);
    }

    let _permit = RESIZING.acquire().await.ok()?;
    // 待っている間に、同じ画像を求めた別のリクエストが作っていることがある
    if let Some(cached) = find_cached(&dir, &stem) {
        return Some(cached);
    }
    let source = source.to_owned();
    let result = tokio::task::spawn_blocking(move || {
        let Some((bytes, extension)) = resize(&source, size)? else {
            return Ok(None);
        };
        std::fs::create_dir_all(&dir)?;
        let target = dir.join(format!("{stem}.{extension}"));
        // 書きかけのファイルを返さないよう、別名で書いてから置き換える
        let tmp = dir.join(format!("{stem}.tmp"));
        std::fs::write(&tmp, bytes)?;
        std::fs::rename(&tmp, &target)?;
        remove_stale(&dir, &prefix, &target);
        Ok::<_, std::io::Error>(Some(target))
    })
    .await
    .expect("画像を縮小するタスクが panic した");
    result.unwrap_or_else(|err| {
        tracing::warn!(album_id, size, error = %err, "cannot resize cover art");
        None
    })
}

fn find_cached(dir: &Path, stem: &str) -> Option<PathBuf> {
    CACHED_EXTENSIONS
        .iter()
        .map(|ext| dir.join(format!("{stem}.{ext}")))
        .find(|path| path.is_file())
}

/// 元の画像を読んで縮小し、JPEG にする。元の画像が `size` 以下なら None。
/// 縮小した JPEG が元より大きければ、元の画像をそのまま返す。返す値は中身と拡張子。
fn resize(source: &Path, size: u32) -> std::io::Result<Option<(Vec<u8>, &'static str)>> {
    let bytes = if is_image(source) {
        std::fs::read(source)?
    } else {
        embedded_picture(source)
            .map(|(_, data)| data)
            .ok_or_else(|| std::io::Error::other("no embedded picture"))?
    };
    let reader = ImageReader::new(Cursor::new(&bytes)).with_guessed_format()?;
    let extension = reader
        .format()
        .and_then(|f| f.extensions_str().first().copied())
        .filter(|ext| CACHED_EXTENSIONS.contains(ext))
        .ok_or_else(|| std::io::Error::other("unknown image format"))?;
    // 大きさはヘッダだけで分かるので、拡大になるなら全体を読まずに済ませる
    let (width, height) = reader.into_dimensions().map_err(std::io::Error::other)?;
    if width.max(height) <= size {
        return Ok(None);
    }
    let image = ImageReader::new(Cursor::new(&bytes))
        .with_guessed_format()?
        .decode()
        .map_err(std::io::Error::other)?
        .resize(size, size, FilterType::CatmullRom);
    let mut jpeg = Vec::new();
    JpegEncoder::new_with_quality(&mut jpeg, JPEG_QUALITY)
        .encode_image(&DynamicImage::ImageRgb8(flatten(&image)))
        .map_err(std::io::Error::other)?;
    // 強く圧縮された元画像を作り直すと、画素が減っても容量が増えることがある
    if jpeg.len() >= bytes.len() {
        return Ok(Some((bytes, extension)));
    }
    Ok(Some((jpeg, "jpg")))
}

/// JPEG は透過を持てないので、白の上に重ねる。
fn flatten(image: &DynamicImage) -> RgbImage {
    let rgba = image.to_rgba8();
    RgbImage::from_fn(rgba.width(), rgba.height(), |x, y| {
        let [r, g, b, a] = rgba.get_pixel(x, y).0;
        let over_white = |c: u8| {
            let (c, a) = (u16::from(c), u16::from(a));
            u8::try_from((c * a + 255 * (255 - a)) / 255).unwrap_or(u8::MAX)
        };
        Rgb([over_white(r), over_white(g), over_white(b)])
    })
}

/// 同じアルバムと大きさの、古い元画像から作ったファイルを消す。書きかけのファイルは残す。
fn remove_stale(dir: &Path, prefix: &str, keep: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let stale = path != keep
            && path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with(prefix) && !n.ends_with(".tmp"));
        if stale {
            let _ = std::fs::remove_file(path);
        }
    }
}

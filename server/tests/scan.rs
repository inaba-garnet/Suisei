//! テスト用の音声を一時ディレクトリに並べてスキャンし、DB の中身を確かめる。

use std::path::{Path, PathBuf};

use axum::body::Body;
use axum::http::Request;
use http_body_util::BodyExt;
use lofty::config::WriteOptions;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::tag::Accessor;
use serde_json::Value;
use suisei::db::{self, Pool};
use suisei::scan::{self, Mode, Scanner};
use suisei::{AppState, Credentials};
use tempfile::TempDir;
use tower::ServiceExt;

struct Library {
    dir: TempDir,
    pool: Pool,
}

impl Library {
    async fn new() -> Self {
        Self {
            dir: tempfile::tempdir().unwrap(),
            pool: db::open_in_memory().await.unwrap(),
        }
    }

    fn path(&self, rel: &str) -> PathBuf {
        self.dir.path().join(rel)
    }

    /// tests/fixtures/tags のファイルを置く。
    fn put(&self, fixture: &str, rel: &str) {
        let src = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/tags")
            .join(fixture);
        let dst = self.path(rel);
        std::fs::create_dir_all(dst.parent().unwrap()).unwrap();
        std::fs::copy(src, dst).unwrap();
    }

    fn set_title(&self, rel: &str, title: &str) {
        let path = self.path(rel);
        let mut file = lofty::read_from_path(&path).unwrap();
        file.primary_tag_mut().unwrap().set_title(title.to_owned());
        file.save_to_path(&path, WriteOptions::default()).unwrap();
    }

    async fn scan(&self) -> scan::Summary {
        scan::run(&self.pool, self.dir.path(), Mode::Quick)
            .await
            .unwrap()
    }

    /// (パス, 曲の ID, 曲名)
    async fn files(&self) -> Vec<(String, String, String)> {
        sqlx::query_as(
            "SELECT file.path, track.id, track.title FROM file
             JOIN track ON track.id = file.track_id ORDER BY file.path",
        )
        .fetch_all(&self.pool)
        .await
        .unwrap()
    }

    async fn track_id(&self, rel: &str) -> String {
        sqlx::query_scalar("SELECT track_id FROM file WHERE path = ?")
            .bind(rel)
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }

    async fn count(&self, table: &str) -> i64 {
        sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT COUNT(*) FROM {table}")))
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }
}

#[tokio::test]
async fn builds_library() {
    let lib = Library::new().await;
    lib.put("full.flac", "歌手A/テストアルバム/03 テスト曲.flac");
    lib.put("full.mp3", "歌手A/テストアルバム/03 テスト曲.mp3");
    lib.put("notag.flac", "未整理/notag.flac");
    lib.put("id3v1.mp3", "id3v1.mp3");
    std::fs::write(lib.path("歌手A/テストアルバム/cover.jpg"), b"").unwrap();
    std::fs::write(lib.path(".hidden.flac"), b"").unwrap();

    let summary = lib.scan().await;
    assert_eq!(summary.files, 4);
    assert_eq!(summary.read, 4);
    assert_eq!(summary.failed, 0);
    // 同じタグの FLAC と MP3 は一曲にまとまる
    assert_eq!(summary.tracks, 3);
    assert_eq!(summary.albums, 3);

    // 配信には可逆圧縮のファイルを使う
    let primary: String = sqlx::query_scalar(
        "SELECT file.path FROM track JOIN file ON file.id = track.primary_file_id
         WHERE track.title = 'テスト曲'",
    )
    .fetch_one(&lib.pool)
    .await
    .unwrap();
    assert_eq!(primary, "歌手A/テストアルバム/03 テスト曲.flac");

    let (album, display_artist, sort_name, year): (String, String, Option<String>, Option<i64>) =
        sqlx::query_as(
            "SELECT name, display_artist, sort_name, year FROM album WHERE name = 'テストアルバム'",
        )
        .fetch_one(&lib.pool)
        .await
        .unwrap();
    assert_eq!(album, "テストアルバム");
    assert_eq!(display_artist, "歌手A");
    assert_eq!(sort_name.as_deref(), Some("テストアルバム"));
    assert_eq!(year, Some(2015));

    let artists: Vec<(String, i64)> = sqlx::query_as(
        "SELECT artist.name, track_artist.position FROM track_artist
         JOIN artist ON artist.id = track_artist.artist_id
         JOIN track ON track.id = track_artist.track_id
         WHERE track.title = 'テスト曲' ORDER BY track_artist.position",
    )
    .fetch_all(&lib.pool)
    .await
    .unwrap();
    assert_eq!(artists, [("歌手A".to_owned(), 0), ("歌手B".to_owned(), 1)]);

    // タグのないファイルは、フォルダ名をアルバム名にする
    let untagged: (String, String) = sqlx::query_as(
        "SELECT track.title, album.name FROM track JOIN album ON album.id = track.album_id
         WHERE track.title = 'notag'",
    )
    .fetch_one(&lib.pool)
    .await
    .unwrap();
    assert_eq!(untagged, ("notag".to_owned(), "未整理".to_owned()));
}

#[tokio::test]
async fn rescan_keeps_ids_without_reading() {
    let lib = Library::new().await;
    lib.put("full.flac", "a/01.flac");
    lib.put("id3v1.mp3", "b/01.mp3");
    lib.scan().await;
    let before = lib.files().await;

    let summary = lib.scan().await;
    assert_eq!(summary.read, 0);
    assert_eq!(lib.files().await, before);
}

#[tokio::test]
async fn moved_file_keeps_track_id() {
    let lib = Library::new().await;
    lib.put("full.flac", "a/01.flac");
    lib.scan().await;
    let id = lib.track_id("a/01.flac").await;

    std::fs::create_dir_all(lib.path("b")).unwrap();
    std::fs::rename(lib.path("a/01.flac"), lib.path("b/01.flac")).unwrap();
    lib.scan().await;
    assert_eq!(lib.track_id("b/01.flac").await, id);
    assert_eq!(lib.count("file").await, 1);
}

#[tokio::test]
async fn edited_tags_keep_track_id() {
    let lib = Library::new().await;
    lib.put("full.flac", "a/01.flac");
    lib.scan().await;
    let id = lib.track_id("a/01.flac").await;

    lib.set_title("a/01.flac", "直したタイトル");
    let summary = lib.scan().await;
    assert_eq!(summary.read, 1);
    assert_eq!(
        lib.files().await,
        [("a/01.flac".to_owned(), id, "直したタイトル".to_owned())]
    );
}

#[tokio::test]
async fn merged_track_leaves_alias() {
    let lib = Library::new().await;
    lib.put("full.flac", "a/01.flac");
    lib.put("full.flac", "a/02.flac");
    lib.set_title("a/02.flac", "別の曲");
    lib.scan().await;
    let kept = lib.track_id("a/01.flac").await;
    let merged = lib.track_id("a/02.flac").await;
    assert_ne!(kept, merged);

    // タグを直したら、もう一方の曲と鍵が一致した
    lib.set_title("a/02.flac", "テスト曲");
    lib.scan().await;
    assert_eq!(lib.track_id("a/02.flac").await, kept);
    assert_eq!(lib.count("track").await, 1);
    let alias: String = sqlx::query_scalar("SELECT new_id FROM id_alias WHERE old_id = ?")
        .bind(&merged)
        .fetch_one(&lib.pool)
        .await
        .unwrap();
    assert_eq!(alias, kept);
}

#[tokio::test]
async fn deleted_file_removes_track() {
    let lib = Library::new().await;
    lib.put("full.flac", "a/01.flac");
    lib.put("id3v1.mp3", "b/01.mp3");
    lib.scan().await;

    std::fs::remove_file(lib.path("b/01.mp3")).unwrap();
    lib.scan().await;
    assert_eq!(lib.count("file").await, 1);
    assert_eq!(lib.count("track").await, 1);
    assert_eq!(lib.count("album").await, 1);
    // 歌手A、歌手B だけが残る
    assert_eq!(lib.count("artist").await, 2);
}

#[tokio::test]
async fn empty_folder_does_not_wipe_library() {
    let lib = Library::new().await;
    lib.put("full.flac", "a/01.flac");
    lib.scan().await;

    std::fs::remove_file(lib.path("a/01.flac")).unwrap();
    let result = scan::run(&lib.pool, lib.dir.path(), Mode::Quick).await;
    assert!(matches!(result, Err(scan::Error::Empty)));
    assert_eq!(lib.count("file").await, 1);
}

#[tokio::test]
async fn missing_folder_is_an_error() {
    let lib = Library::new().await;
    let result = scan::run(&lib.pool, &lib.path("missing"), Mode::Quick).await;
    assert!(matches!(result, Err(scan::Error::Folder(_))));
}

#[tokio::test]
async fn unreadable_file_is_kept() {
    let lib = Library::new().await;
    lib.put("full.flac", "a/01.flac");
    lib.scan().await;
    let before = lib.files().await;

    // 壊れたファイルに置き換える。一時的に読めないだけかもしれないので、前回の行を残す
    std::fs::write(lib.path("a/01.flac"), b"broken").unwrap();
    let summary = lib.scan().await;
    assert_eq!(summary.failed, 1);
    assert_eq!(lib.files().await, before);
}

#[tokio::test]
async fn scan_endpoints() {
    let lib = Library::new().await;
    lib.put("full.flac", "a/01.flac");
    lib.put("id3v1.mp3", "b/01.mp3");
    let scanner = Scanner::new(lib.pool.clone(), lib.dir.path().to_owned());
    let app = suisei::router(AppState {
        credentials: Credentials {
            user: "inaba".into(),
            password: "sesame".into(),
            api_key: None,
        },
        db: lib.pool.clone(),
        scanner: scanner.clone(),
    });
    let status = |endpoint: &'static str| {
        let app = app.clone();
        async move {
            let uri = format!("/rest/{endpoint}?u=inaba&p=sesame&f=json");
            let res = app
                .oneshot(Request::get(uri).body(Body::empty()).unwrap())
                .await
                .unwrap();
            let body = res.into_body().collect().await.unwrap().to_bytes();
            let value: Value = serde_json::from_slice(&body).unwrap();
            value["subsonic-response"]["scanStatus"].clone()
        }
    };

    let before = status("getScanStatus").await;
    assert_eq!(before["scanning"], false);
    assert_eq!(before["count"], 0);
    assert!(before.get("lastScan").is_none());

    // 始めた直後の応答から、スキャン中として返す
    assert_eq!(status("startScan").await["scanning"], true);
    scanner.wait().await;

    let after = status("getScanStatus").await;
    assert_eq!(after["scanning"], false);
    assert_eq!(after["count"], 2);
    assert_eq!(after["folderCount"], 2);
    assert!(after["lastScan"].as_str().unwrap().ends_with('Z'));
    assert_eq!(lib.count("track").await, 2);
}

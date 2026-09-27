//! スキャンが読み書きするライブラリの表。

use std::collections::HashSet;

use super::{Connection, Pool};

/// 音楽フォルダは一つだけ持つ。
pub const MUSIC_FOLDER_ID: i64 = 1;

/// file 表の一行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileRow {
    pub id: String,
    pub path: String,
    pub size: i64,
    pub mtime: i64,
    pub track_id: String,
    pub suffix: String,
    pub content_type: String,
    pub duration_ms: i64,
    pub bit_rate: Option<i64>,
    pub sample_rate: Option<i64>,
    pub channels: Option<i64>,
    pub bit_depth: Option<i64>,
    pub lossless: bool,
    /// 生のタグの JSON
    pub tags: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtistRow {
    pub id: String,
    pub match_key: String,
    pub name: String,
    pub sort_name: Option<String>,
    pub sort_name_source: Option<String>,
    pub sort_key: String,
    pub search_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlbumRow {
    pub id: String,
    pub match_key: String,
    pub name: String,
    pub display_artist: String,
    pub sort_name: Option<String>,
    pub sort_name_source: Option<String>,
    pub sort_key: String,
    pub search_text: String,
    pub year: Option<i64>,
    pub created_at: i64,
    pub compilation: bool,
    pub cover_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackRow {
    pub id: String,
    pub match_key: String,
    pub album_id: String,
    pub title: String,
    pub display_artist: String,
    pub sort_name: Option<String>,
    pub sort_name_source: Option<String>,
    pub sort_key: String,
    pub search_text: String,
    pub disc_number: Option<i64>,
    pub track_number: Option<i64>,
    pub year: Option<i64>,
    pub primary_file_id: String,
    pub created_at: i64,
}

/// track_artist と album_artist の一行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreditRow {
    pub owner_id: String,
    pub position: i64,
    pub artist_id: String,
    pub credited_name: String,
    pub credited_sort: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenreRow {
    pub track_id: String,
    pub position: i64,
    pub genre: String,
}

/// スキャンの前の DB の状態。ID を引き継ぐのに使う。
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    pub files: Vec<FileRow>,
    /// (id, match_key, album_id, created_at)
    pub tracks: Vec<(String, String, String, i64)>,
    /// (id, match_key, created_at)
    pub albums: Vec<(String, String, i64)>,
    /// (id, match_key)
    pub artists: Vec<(String, String)>,
    /// (track_id, position, artist_id)
    pub track_artists: Vec<(String, i64, String)>,
    /// (album_id, position, artist_id)
    pub album_artists: Vec<(String, i64, String)>,
    /// 別名として使っている古い ID
    pub alias_ids: Vec<String>,
}

/// スキャンの結果として書き込む、ライブラリ全体の状態。
#[derive(Debug, Clone, Default)]
pub struct Library {
    pub music_folder_path: String,
    pub artists: Vec<ArtistRow>,
    pub albums: Vec<AlbumRow>,
    pub tracks: Vec<TrackRow>,
    pub files: Vec<FileRow>,
    pub track_artists: Vec<CreditRow>,
    pub album_artists: Vec<CreditRow>,
    pub track_genres: Vec<GenreRow>,
    /// (old_id, new_id)
    pub aliases: Vec<(String, String)>,
}

pub async fn snapshot(pool: &Pool) -> Result<Snapshot, sqlx::Error> {
    let mut conn = pool.acquire().await?;
    let files = sqlx::query_as!(
        FileRow,
        r#"SELECT id AS "id!", path, size, mtime, track_id, suffix, content_type, duration_ms,
                  bit_rate, sample_rate, channels, bit_depth, lossless, tags
           FROM file WHERE music_folder_id = ?"#,
        MUSIC_FOLDER_ID
    )
    .fetch_all(&mut *conn)
    .await?;
    let tracks = sqlx::query!(r#"SELECT id AS "id!", match_key, album_id, created_at FROM track"#)
        .fetch_all(&mut *conn)
        .await?
        .into_iter()
        .map(|r| (r.id, r.match_key, r.album_id, r.created_at))
        .collect();
    let albums = sqlx::query!(r#"SELECT id AS "id!", match_key, created_at FROM album"#)
        .fetch_all(&mut *conn)
        .await?
        .into_iter()
        .map(|r| (r.id, r.match_key, r.created_at))
        .collect();
    let artists = sqlx::query!(r#"SELECT id AS "id!", match_key FROM artist"#)
        .fetch_all(&mut *conn)
        .await?
        .into_iter()
        .map(|r| (r.id, r.match_key))
        .collect();
    let track_artists = sqlx::query!("SELECT track_id, position, artist_id FROM track_artist")
        .fetch_all(&mut *conn)
        .await?
        .into_iter()
        .map(|r| (r.track_id, r.position, r.artist_id))
        .collect();
    let album_artists = sqlx::query!("SELECT album_id, position, artist_id FROM album_artist")
        .fetch_all(&mut *conn)
        .await?
        .into_iter()
        .map(|r| (r.album_id, r.position, r.artist_id))
        .collect();
    let alias_ids = sqlx::query_scalar!(r#"SELECT old_id AS "old_id!" FROM id_alias"#)
        .fetch_all(&mut *conn)
        .await?;
    Ok(Snapshot {
        files,
        tracks,
        albums,
        artists,
        track_artists,
        album_artists,
        alias_ids,
    })
}

/// ライブラリ全体を一つのトランザクションで書き込む。
/// 行を消して入れ直すと、曲を参照する表（プレイリストなど）の行まで連鎖して消えるので、
/// 残る行は更新し、なくなった行だけを消す。
pub async fn replace(pool: &Pool, library: &Library) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    sqlx::query!(
        "INSERT INTO music_folder (id, name, path) VALUES (?, 'Music', ?)
         ON CONFLICT (id) DO UPDATE SET path = excluded.path",
        MUSIC_FOLDER_ID,
        library.music_folder_path
    )
    .execute(&mut *tx)
    .await?;

    // 行を消す前に、それを参照する表を空にする。下で入れ直す
    sqlx::query!("DELETE FROM track_artist")
        .execute(&mut *tx)
        .await?;
    sqlx::query!("DELETE FROM album_artist")
        .execute(&mut *tx)
        .await?;
    sqlx::query!("DELETE FROM track_genre")
        .execute(&mut *tx)
        .await?;

    for a in &library.artists {
        sqlx::query!(
            "INSERT INTO artist (id, match_key, name, sort_name, sort_name_source, sort_key,
                 search_text)
             VALUES (?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT (id) DO UPDATE SET match_key = excluded.match_key, name = excluded.name,
                 sort_name = excluded.sort_name, sort_name_source = excluded.sort_name_source,
                 sort_key = excluded.sort_key, search_text = excluded.search_text",
            a.id,
            a.match_key,
            a.name,
            a.sort_name,
            a.sort_name_source,
            a.sort_key,
            a.search_text
        )
        .execute(&mut *tx)
        .await?;
    }
    for a in &library.albums {
        sqlx::query!(
            "INSERT INTO album (id, match_key, name, display_artist, sort_name, sort_name_source,
                 year, created_at, sort_key, compilation, search_text, cover_path)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT (id) DO UPDATE SET match_key = excluded.match_key, name = excluded.name,
                 display_artist = excluded.display_artist, sort_name = excluded.sort_name,
                 sort_name_source = excluded.sort_name_source, year = excluded.year,
                 sort_key = excluded.sort_key, compilation = excluded.compilation,
                 created_at = excluded.created_at, search_text = excluded.search_text,
                 cover_path = excluded.cover_path",
            a.id,
            a.match_key,
            a.name,
            a.display_artist,
            a.sort_name,
            a.sort_name_source,
            a.year,
            a.created_at,
            a.sort_key,
            a.compilation,
            a.search_text,
            a.cover_path
        )
        .execute(&mut *tx)
        .await?;
    }
    for t in &library.tracks {
        sqlx::query!(
            "INSERT INTO track (id, match_key, album_id, title, display_artist, sort_name,
                 sort_name_source, disc_number, track_number, year, primary_file_id, sort_key,
                 created_at, search_text)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT (id) DO UPDATE SET match_key = excluded.match_key,
                 album_id = excluded.album_id, title = excluded.title,
                 display_artist = excluded.display_artist, sort_name = excluded.sort_name,
                 sort_name_source = excluded.sort_name_source,
                 disc_number = excluded.disc_number, track_number = excluded.track_number,
                 year = excluded.year, primary_file_id = excluded.primary_file_id,
                 sort_key = excluded.sort_key, created_at = excluded.created_at,
                 search_text = excluded.search_text",
            t.id,
            t.match_key,
            t.album_id,
            t.title,
            t.display_artist,
            t.sort_name,
            t.sort_name_source,
            t.disc_number,
            t.track_number,
            t.year,
            t.primary_file_id,
            t.sort_key,
            t.created_at,
            t.search_text
        )
        .execute(&mut *tx)
        .await?;
    }

    // 消えたパスの行を先に消し、同じパスに新しい行を入れるときに一意制約に当たらないようにする
    let keep: HashSet<&str> = library.files.iter().map(|f| f.id.as_str()).collect();
    let stored = sqlx::query_scalar!(r#"SELECT id AS "id!" FROM file"#)
        .fetch_all(&mut *tx)
        .await?;
    for id in stored.iter().filter(|id| !keep.contains(&id.as_str())) {
        sqlx::query!("DELETE FROM file WHERE id = ?", id)
            .execute(&mut *tx)
            .await?;
    }
    for f in &library.files {
        sqlx::query!(
            "INSERT INTO file (id, music_folder_id, path, size, mtime, track_id, suffix,
                 content_type, duration_ms, bit_rate, sample_rate, channels, bit_depth, lossless,
                 tags)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT (id) DO UPDATE SET path = excluded.path, size = excluded.size,
                 mtime = excluded.mtime, track_id = excluded.track_id, suffix = excluded.suffix,
                 content_type = excluded.content_type, duration_ms = excluded.duration_ms,
                 bit_rate = excluded.bit_rate, sample_rate = excluded.sample_rate,
                 channels = excluded.channels, bit_depth = excluded.bit_depth,
                 lossless = excluded.lossless, tags = excluded.tags",
            f.id,
            MUSIC_FOLDER_ID,
            f.path,
            f.size,
            f.mtime,
            f.track_id,
            f.suffix,
            f.content_type,
            f.duration_ms,
            f.bit_rate,
            f.sample_rate,
            f.channels,
            f.bit_depth,
            f.lossless,
            f.tags
        )
        .execute(&mut *tx)
        .await?;
    }

    for c in &library.track_artists {
        sqlx::query!(
            "INSERT INTO track_artist (track_id, position, artist_id, credited_name, credited_sort)
             VALUES (?, ?, ?, ?, ?)",
            c.owner_id,
            c.position,
            c.artist_id,
            c.credited_name,
            c.credited_sort
        )
        .execute(&mut *tx)
        .await?;
    }
    for c in &library.album_artists {
        sqlx::query!(
            "INSERT INTO album_artist (album_id, position, artist_id, credited_name, credited_sort)
             VALUES (?, ?, ?, ?, ?)",
            c.owner_id,
            c.position,
            c.artist_id,
            c.credited_name,
            c.credited_sort
        )
        .execute(&mut *tx)
        .await?;
    }
    for g in &library.track_genres {
        sqlx::query!(
            "INSERT INTO track_genre (track_id, position, genre) VALUES (?, ?, ?)",
            g.track_id,
            g.position,
            g.genre
        )
        .execute(&mut *tx)
        .await?;
    }

    // マージで消える曲の再生履歴を、残る曲へ付け替える。消える曲の行は下で連鎖して消える
    for (old_id, new_id) in &library.aliases {
        sqlx::query!(
            "INSERT INTO play_history (track_id, played_at)
             SELECT ?, played_at FROM play_history WHERE track_id = ?
             ON CONFLICT (track_id, played_at) DO NOTHING",
            new_id,
            old_id
        )
        .execute(&mut *tx)
        .await?;
    }

    // マージで消える曲を、プレイリストの同じ位置のまま残る曲に付け替える
    for (old_id, new_id) in &library.aliases {
        sqlx::query!(
            "UPDATE playlist_entry SET track_id = ? WHERE track_id = ?",
            new_id,
            old_id
        )
        .execute(&mut *tx)
        .await?;
    }

    // マージで消える曲、アルバム、アーティストのお気に入りと評価を、残る側へ引き継ぐ。
    // お気に入りの日時は古いほう、評価は残る側を優先する（docs/schema.md）
    for (old_id, new_id) in &library.aliases {
        carry_annotation(&mut tx, old_id, new_id).await?;
    }

    // 参照がなくなった行を消す。file と track_artist などは入れ直したので、残っているのは使われない行
    sqlx::query!("DELETE FROM track WHERE id NOT IN (SELECT track_id FROM file)")
        .execute(&mut *tx)
        .await?;
    sqlx::query!("DELETE FROM album WHERE id NOT IN (SELECT album_id FROM track)")
        .execute(&mut *tx)
        .await?;
    sqlx::query!(
        "DELETE FROM artist WHERE id NOT IN (SELECT artist_id FROM track_artist)
             AND id NOT IN (SELECT artist_id FROM album_artist)"
    )
    .execute(&mut *tx)
    .await?;

    for (old_id, new_id) in &library.aliases {
        // 消える ID を指していた別名も、新しい ID に付け替える
        sqlx::query!(
            "UPDATE id_alias SET new_id = ? WHERE new_id = ?",
            new_id,
            old_id
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query!(
            "INSERT INTO id_alias (old_id, new_id) VALUES (?, ?)
             ON CONFLICT (old_id) DO UPDATE SET new_id = excluded.new_id",
            old_id,
            new_id
        )
        .execute(&mut *tx)
        .await?;
    }

    // 指す先が消えた別名を消す
    sqlx::query!(
        "DELETE FROM id_alias WHERE new_id NOT IN (SELECT id FROM track)
             AND new_id NOT IN (SELECT id FROM album) AND new_id NOT IN (SELECT id FROM artist)"
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await
}

/// 消える行のお気に入りと評価を、残る行へ引き継ぐ。ID は種類をまたいで一意なので、三つの表を順に試す。
async fn carry_annotation(
    tx: &mut Connection,
    old_id: &str,
    new_id: &str,
) -> Result<(), sqlx::Error> {
    // 残る側にお気に入りがなければ消える側の日時を、両方にあれば古いほうを採る
    if let Some(old) = sqlx::query!("SELECT starred_at, rating FROM track WHERE id = ?", old_id)
        .fetch_optional(&mut *tx)
        .await?
    {
        sqlx::query!(
            "UPDATE track SET
                 starred_at = CASE WHEN starred_at IS NULL OR starred_at > ? THEN ?
                                   ELSE starred_at END,
                 rating = COALESCE(rating, ?)
             WHERE id = ?",
            old.starred_at,
            old.starred_at,
            old.rating,
            new_id
        )
        .execute(&mut *tx)
        .await?;
    }
    if let Some(old) = sqlx::query!("SELECT starred_at, rating FROM album WHERE id = ?", old_id)
        .fetch_optional(&mut *tx)
        .await?
    {
        sqlx::query!(
            "UPDATE album SET
                 starred_at = CASE WHEN starred_at IS NULL OR starred_at > ? THEN ?
                                   ELSE starred_at END,
                 rating = COALESCE(rating, ?)
             WHERE id = ?",
            old.starred_at,
            old.starred_at,
            old.rating,
            new_id
        )
        .execute(&mut *tx)
        .await?;
    }
    if let Some(old) = sqlx::query!("SELECT starred_at, rating FROM artist WHERE id = ?", old_id)
        .fetch_optional(&mut *tx)
        .await?
    {
        sqlx::query!(
            "UPDATE artist SET
                 starred_at = CASE WHEN starred_at IS NULL OR starred_at > ? THEN ?
                                   ELSE starred_at END,
                 rating = COALESCE(rating, ?)
             WHERE id = ?",
            old.starred_at,
            old.starred_at,
            old.rating,
            new_id
        )
        .execute(&mut *tx)
        .await?;
    }
    Ok(())
}

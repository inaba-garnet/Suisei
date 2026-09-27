//! 閲覧のエンドポイントが読む問い合わせ。

use super::Pool;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtistEntry {
    pub id: String,
    pub name: String,
    pub sort_name: Option<String>,
    pub sort_key: String,
    pub album_count: i64,
}

/// アルバムアーティストになっているアーティストを、並べ替えキーの順に返す。
/// キーはバイト列の順に並べる前提で作っている。PostgreSQL に移るときは `COLLATE "C"` を付ける。
pub async fn album_artists(pool: &Pool) -> Result<Vec<ArtistEntry>, sqlx::Error> {
    sqlx::query_as!(
        ArtistEntry,
        r#"SELECT artist.id AS "id!", artist.name, artist.sort_name, artist.sort_key,
                  COUNT(DISTINCT album_artist.album_id) AS "album_count!: i64"
           FROM artist JOIN album_artist ON album_artist.artist_id = artist.id
           GROUP BY artist.id
           ORDER BY artist.sort_key, artist.id"#
    )
    .fetch_all(pool)
    .await
}

/// 手動や自動のマージで消えた ID なら、引き継いだ ID に引き直す。
pub async fn resolve_alias(pool: &Pool, id: &str) -> Result<String, sqlx::Error> {
    let new_id = sqlx::query_scalar!("SELECT new_id FROM id_alias WHERE old_id = ?", id)
        .fetch_optional(pool)
        .await?;
    Ok(new_id.unwrap_or_else(|| id.to_owned()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Artist {
    pub id: String,
    pub name: String,
    pub sort_name: Option<String>,
}

pub async fn artist(pool: &Pool, id: &str) -> Result<Option<Artist>, sqlx::Error> {
    sqlx::query_as!(
        Artist,
        r#"SELECT id AS "id!", name, sort_name FROM artist WHERE id = ?"#,
        id
    )
    .fetch_optional(pool)
    .await
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Album {
    pub id: String,
    pub name: String,
    pub display_artist: String,
    pub sort_name: Option<String>,
    pub year: Option<i64>,
    pub created_at: i64,
    pub compilation: bool,
    pub song_count: i64,
    /// 配信ファイルの長さの合計
    pub duration_ms: i64,
}

/// アルバムアーティストのアルバムに、曲で参加しているアルバムを足す。年の古い順、同じ年なら並べ替えキーの順。
pub async fn albums_of_artist(pool: &Pool, artist_id: &str) -> Result<Vec<Album>, sqlx::Error> {
    sqlx::query_as!(
        Album,
        r#"SELECT album.id AS "id!", album.name, album.display_artist, album.sort_name, album.year,
                  album.created_at, album.compilation,
                  (SELECT COUNT(*) FROM track WHERE track.album_id = album.id) AS "song_count!: i64",
                  (SELECT COALESCE(SUM(file.duration_ms), 0) FROM track
                     JOIN file ON file.id = track.primary_file_id
                     WHERE track.album_id = album.id) AS "duration_ms!: i64"
           FROM album
           WHERE album.id IN (
               SELECT album_id FROM album_artist WHERE artist_id = ?
               UNION
               SELECT track.album_id FROM track
                 JOIN track_artist ON track_artist.track_id = track.id
                 WHERE track_artist.artist_id = ?)
           ORDER BY album.year IS NULL, album.year, album.sort_key, album.id"#,
        artist_id,
        artist_id
    )
    .fetch_all(pool)
    .await
}

pub async fn album(pool: &Pool, id: &str) -> Result<Option<Album>, sqlx::Error> {
    sqlx::query_as!(
        Album,
        r#"SELECT album.id AS "id!", album.name, album.display_artist, album.sort_name, album.year,
                  album.created_at, album.compilation,
                  (SELECT COUNT(*) FROM track WHERE track.album_id = album.id) AS "song_count!: i64",
                  (SELECT COALESCE(SUM(file.duration_ms), 0) FROM track
                     JOIN file ON file.id = track.primary_file_id
                     WHERE track.album_id = album.id) AS "duration_ms!: i64"
           FROM album WHERE album.id = ?"#,
        id
    )
    .fetch_optional(pool)
    .await
}

/// 曲やアルバムに付くアーティスト。名前は表記ゆれをまとめた表示名。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Credit {
    pub owner_id: String,
    pub artist_id: String,
    pub name: String,
}

pub async fn credits_of_album(pool: &Pool, album_id: &str) -> Result<Vec<Credit>, sqlx::Error> {
    sqlx::query_as!(
        Credit,
        r#"SELECT album_artist.album_id AS owner_id, artist.id AS "artist_id!", artist.name
           FROM album_artist JOIN artist ON artist.id = album_artist.artist_id
           WHERE album_artist.album_id = ?
           ORDER BY album_artist.position"#,
        album_id
    )
    .fetch_all(pool)
    .await
}

/// アルバムの曲すべてのアーティスト。
pub async fn track_artists_of_album(
    pool: &Pool,
    album_id: &str,
) -> Result<Vec<Credit>, sqlx::Error> {
    sqlx::query_as!(
        Credit,
        r#"SELECT track_artist.track_id AS owner_id, artist.id AS "artist_id!", artist.name
           FROM track_artist
             JOIN artist ON artist.id = track_artist.artist_id
             JOIN track ON track.id = track_artist.track_id
           WHERE track.album_id = ?
           ORDER BY track_artist.track_id, track_artist.position"#,
        album_id
    )
    .fetch_all(pool)
    .await
}

/// アルバムの曲すべてのジャンル。(曲の ID, ジャンル)
pub async fn track_genres_of_album(
    pool: &Pool,
    album_id: &str,
) -> Result<Vec<(String, String)>, sqlx::Error> {
    let rows = sqlx::query!(
        "SELECT track_genre.track_id, track_genre.genre FROM track_genre
           JOIN track ON track.id = track_genre.track_id
         WHERE track.album_id = ?
         ORDER BY track_genre.track_id, track_genre.position",
        album_id
    )
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|r| (r.track_id, r.genre)).collect())
}

/// アルバムのジャンルを、曲数の多い順に返す。
pub async fn album_genres(pool: &Pool, album_id: &str) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar!(
        r#"SELECT track_genre.genre AS "genre!" FROM track_genre
             JOIN track ON track.id = track_genre.track_id
           WHERE track.album_id = ?
           GROUP BY track_genre.genre
           ORDER BY COUNT(*) DESC, track_genre.genre"#,
        album_id
    )
    .fetch_all(pool)
    .await
}

/// 曲と、その配信ファイル。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Song {
    pub id: String,
    pub album_id: String,
    pub title: String,
    pub display_artist: String,
    pub sort_name: Option<String>,
    pub disc_number: Option<i64>,
    pub track_number: Option<i64>,
    pub year: Option<i64>,
    pub created_at: i64,
    pub album_name: String,
    pub album_display_artist: String,
    pub path: String,
    pub size: i64,
    pub suffix: String,
    pub content_type: String,
    pub duration_ms: i64,
    pub bit_rate: Option<i64>,
    pub sample_rate: Option<i64>,
    pub channels: Option<i64>,
    pub bit_depth: Option<i64>,
}

/// ディスク番号、トラック番号、並べ替えキーの順。
pub async fn songs_of_album(pool: &Pool, album_id: &str) -> Result<Vec<Song>, sqlx::Error> {
    sqlx::query_as!(
        Song,
        r#"SELECT track.id AS "id!", track.album_id, track.title, track.display_artist,
                  track.sort_name, track.disc_number, track.track_number, track.year,
                  track.created_at, album.name AS album_name,
                  album.display_artist AS album_display_artist, file.path, file.size,
                  file.suffix, file.content_type, file.duration_ms, file.bit_rate,
                  file.sample_rate, file.channels, file.bit_depth
           FROM track
             JOIN album ON album.id = track.album_id
             JOIN file ON file.id = track.primary_file_id
           WHERE track.album_id = ?
           ORDER BY track.disc_number IS NULL, track.disc_number,
                    track.track_number IS NULL, track.track_number, track.sort_key, track.id"#,
        album_id
    )
    .fetch_all(pool)
    .await
}

pub async fn song(pool: &Pool, id: &str) -> Result<Option<Song>, sqlx::Error> {
    sqlx::query_as!(
        Song,
        r#"SELECT track.id AS "id!", track.album_id, track.title, track.display_artist,
                  track.sort_name, track.disc_number, track.track_number, track.year,
                  track.created_at, album.name AS album_name,
                  album.display_artist AS album_display_artist, file.path, file.size,
                  file.suffix, file.content_type, file.duration_ms, file.bit_rate,
                  file.sample_rate, file.channels, file.bit_depth
           FROM track
             JOIN album ON album.id = track.album_id
             JOIN file ON file.id = track.primary_file_id
           WHERE track.id = ?"#,
        id
    )
    .fetch_optional(pool)
    .await
}

//! お気に入りと評価の問い合わせ（docs/schema.md の「お気に入りと評価」）。
//! ID は種類をまたいで一意なので、曲、アルバム、アーティストの表を順に試す。

use super::Pool;
use super::browse::ArtistSummary;

/// お気に入りにする。お気に入りにし直しても日時は変えない。ID がなければ false。
pub async fn star(pool: &Pool, id: &str, at: i64) -> Result<bool, sqlx::Error> {
    let mut rows = 0;
    rows += sqlx::query!(
        "UPDATE track SET starred_at = COALESCE(starred_at, ?) WHERE id = ?",
        at,
        id
    )
    .execute(pool)
    .await?
    .rows_affected();
    rows += sqlx::query!(
        "UPDATE album SET starred_at = COALESCE(starred_at, ?) WHERE id = ?",
        at,
        id
    )
    .execute(pool)
    .await?
    .rows_affected();
    rows += sqlx::query!(
        "UPDATE artist SET starred_at = COALESCE(starred_at, ?) WHERE id = ?",
        at,
        id
    )
    .execute(pool)
    .await?
    .rows_affected();
    Ok(rows > 0)
}

/// お気に入りから外す。ID がなければ false。
pub async fn unstar(pool: &Pool, id: &str) -> Result<bool, sqlx::Error> {
    let mut rows = 0;
    rows += sqlx::query!("UPDATE track SET starred_at = NULL WHERE id = ?", id)
        .execute(pool)
        .await?
        .rows_affected();
    rows += sqlx::query!("UPDATE album SET starred_at = NULL WHERE id = ?", id)
        .execute(pool)
        .await?
        .rows_affected();
    rows += sqlx::query!("UPDATE artist SET starred_at = NULL WHERE id = ?", id)
        .execute(pool)
        .await?
        .rows_affected();
    Ok(rows > 0)
}

/// 評価を付ける。`None` なら消す。ID がなければ false。
pub async fn set_rating(pool: &Pool, id: &str, rating: Option<i64>) -> Result<bool, sqlx::Error> {
    let mut rows = 0;
    rows += sqlx::query!("UPDATE track SET rating = ? WHERE id = ?", rating, id)
        .execute(pool)
        .await?
        .rows_affected();
    rows += sqlx::query!("UPDATE album SET rating = ? WHERE id = ?", rating, id)
        .execute(pool)
        .await?
        .rows_affected();
    rows += sqlx::query!("UPDATE artist SET rating = ? WHERE id = ?", rating, id)
        .execute(pool)
        .await?
        .rows_affected();
    Ok(rows > 0)
}

/// お気に入りの曲の ID。お気に入りにした新しい順。
pub async fn starred_songs(pool: &Pool) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar!(
        r#"SELECT id AS "id!" FROM track WHERE starred_at IS NOT NULL
           ORDER BY starred_at DESC, id"#
    )
    .fetch_all(pool)
    .await
}

/// お気に入りのアルバムの ID。お気に入りにした新しい順。
pub async fn starred_albums(pool: &Pool) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar!(
        r#"SELECT id AS "id!" FROM album WHERE starred_at IS NOT NULL
           ORDER BY starred_at DESC, id"#
    )
    .fetch_all(pool)
    .await
}

/// お気に入りのアーティスト。お気に入りにした新しい順。
pub async fn starred_artists(pool: &Pool) -> Result<Vec<ArtistSummary>, sqlx::Error> {
    sqlx::query_as!(
        ArtistSummary,
        r#"SELECT artist.id AS "id!", artist.name, artist.sort_name,
                  (SELECT COUNT(*) FROM (
                      SELECT album_id FROM album_artist WHERE artist_id = artist.id
                      UNION
                      SELECT track.album_id FROM track
                        JOIN track_artist ON track_artist.track_id = track.id
                        WHERE track_artist.artist_id = artist.id
                      UNION
                      SELECT track.album_id FROM track
                        JOIN track_contributor ON track_contributor.track_id = track.id
                        WHERE track_contributor.artist_id = artist.id)) AS "album_count!: i64",
                  (CASE WHEN EXISTS (SELECT 1 FROM album_artist
                                     WHERE album_artist.artist_id = artist.id)
                     THEN 'albumartist ' ELSE '' END
                   || CASE WHEN EXISTS (SELECT 1 FROM track_artist
                                        WHERE track_artist.artist_id = artist.id)
                      THEN 'artist ' ELSE '' END
                   || CASE WHEN EXISTS (SELECT 1 FROM track_contributor
                                        WHERE track_contributor.artist_id = artist.id
                                          AND track_contributor.role = 'composer')
                      THEN 'composer ' ELSE '' END
                   || CASE WHEN EXISTS (SELECT 1 FROM track_contributor
                                        WHERE track_contributor.artist_id = artist.id
                                          AND track_contributor.role = 'lyricist')
                      THEN 'lyricist ' ELSE '' END
                   || CASE WHEN EXISTS (SELECT 1 FROM track_contributor
                                        WHERE track_contributor.artist_id = artist.id
                                          AND track_contributor.role = 'arranger')
                      THEN 'arranger ' ELSE '' END) AS "roles!: String",
                  artist.starred_at, artist.rating
           FROM artist WHERE artist.starred_at IS NOT NULL
           ORDER BY artist.starred_at DESC, artist.id"#
    )
    .fetch_all(pool)
    .await
}

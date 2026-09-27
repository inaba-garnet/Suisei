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

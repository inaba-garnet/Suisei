//! 再生履歴の問い合わせ（docs/schema.md の「再生履歴」）。

use super::Pool;

/// 曲の再生を一回記録する。曲がなければ false。同じ曲と同じ時刻の組は一度だけ記録する。
pub async fn record(pool: &Pool, track_id: &str, played_at: i64) -> Result<bool, sqlx::Error> {
    let exists = sqlx::query_scalar!("SELECT id FROM track WHERE id = ?", track_id)
        .fetch_optional(pool)
        .await?
        .is_some();
    if exists {
        sqlx::query!(
            "INSERT INTO play_history (track_id, played_at) VALUES (?, ?)
             ON CONFLICT (track_id, played_at) DO NOTHING",
            track_id,
            played_at
        )
        .execute(pool)
        .await?;
    }
    Ok(exists)
}

/// その名前のアーティストが参加している曲のうち、再生したものを再生回数の多い順に返す。
pub async fn top_songs(pool: &Pool, artist: &str, count: i64) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar!(
        r#"SELECT track.id AS "id!" FROM track
             JOIN play_history ON play_history.track_id = track.id
           WHERE track.id IN (
               SELECT track_artist.track_id FROM track_artist
                 JOIN artist ON artist.id = track_artist.artist_id
               WHERE artist.name = ?)
           GROUP BY track.id
           ORDER BY COUNT(*) DESC, MAX(play_history.played_at) DESC, track.id
           LIMIT ?"#,
        artist,
        count
    )
    .fetch_all(pool)
    .await
}

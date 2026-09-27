//! プレイリストの問い合わせ（docs/schema.md の「プレイリスト」）。

use super::{IdKind, Pool, new_id};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Playlist {
    pub id: String,
    pub name: String,
    pub comment: Option<String>,
    pub public: bool,
    pub created_at: i64,
    pub changed_at: i64,
    pub song_count: i64,
    /// 配信ファイルの長さの合計
    pub duration_ms: i64,
    /// 画像のある最初の曲のアルバム
    pub cover_album_id: Option<String>,
}

/// プレイリストと、曲数、長さの合計、カバーアートを読む。`$tail` に絞り込みと並び順を書く。
macro_rules! playlists {
    ($tail:literal $(, $arg:expr)* $(,)?) => {
        sqlx::query_as!(
            Playlist,
            r#"SELECT playlist.id AS "id!", playlist.name, playlist.comment, playlist.public,
                      playlist.created_at, playlist.changed_at,
                      (SELECT COUNT(*) FROM playlist_entry
                         WHERE playlist_entry.playlist_id = playlist.id) AS "song_count!: i64",
                      (SELECT COALESCE(SUM(file.duration_ms), 0) FROM playlist_entry
                         JOIN track ON track.id = playlist_entry.track_id
                         JOIN file ON file.id = track.primary_file_id
                         WHERE playlist_entry.playlist_id = playlist.id) AS "duration_ms!: i64",
                      (SELECT album.id FROM playlist_entry
                         JOIN track ON track.id = playlist_entry.track_id
                         JOIN album ON album.id = track.album_id
                         WHERE playlist_entry.playlist_id = playlist.id
                           AND album.cover_path IS NOT NULL
                         ORDER BY playlist_entry.position LIMIT 1) AS cover_album_id
               FROM playlist "# + $tail
            $(, $arg)*
        )
    };
}

/// 作った順。名前で並べると DB の照合順序に頼ることになるので避ける（docs/server.md）。
pub async fn list(pool: &Pool) -> Result<Vec<Playlist>, sqlx::Error> {
    playlists!("ORDER BY playlist.created_at, playlist.id")
        .fetch_all(pool)
        .await
}

pub async fn get(pool: &Pool, id: &str) -> Result<Option<Playlist>, sqlx::Error> {
    playlists!("WHERE playlist.id = ?", id)
        .fetch_optional(pool)
        .await
}

/// 曲の ID を並びの順に返す。同じ曲が何度も出ることがある。
pub async fn entries(pool: &Pool, id: &str) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar!(
        "SELECT track_id FROM playlist_entry WHERE playlist_id = ? ORDER BY position",
        id
    )
    .fetch_all(pool)
    .await
}

/// ライブラリにある曲だけを、順序と重複を保って返す。
pub async fn existing_tracks(pool: &Pool, ids: &[String]) -> Result<Vec<String>, sqlx::Error> {
    let mut found = Vec::with_capacity(ids.len());
    for id in ids {
        let exists = sqlx::query_scalar!("SELECT id FROM track WHERE id = ?", id)
            .fetch_optional(pool)
            .await?
            .is_some();
        if exists {
            found.push(id.clone());
        }
    }
    Ok(found)
}

/// 空のプレイリストを作り、その ID を返す。
pub async fn create(pool: &Pool, name: &str, now: i64) -> Result<String, sqlx::Error> {
    loop {
        let id = new_id(IdKind::Playlist);
        let inserted = sqlx::query!(
            "INSERT INTO playlist (id, name, comment, public, created_at, changed_at)
             VALUES (?, ?, NULL, FALSE, ?, ?)
             ON CONFLICT (id) DO NOTHING",
            id,
            name,
            now,
            now
        )
        .execute(pool)
        .await?
        .rows_affected();
        // 乱数の ID が既存と重なったら採り直す
        if inserted > 0 {
            return Ok(id);
        }
    }
}

/// 名前、コメント、公開のうち、渡したものだけを変える。
#[derive(Debug, Clone, Default)]
pub struct Changes {
    pub name: Option<String>,
    pub comment: Option<String>,
    pub public: Option<bool>,
}

/// 属性と曲の並びを変える。`tracks` を渡すと並びをそれで置き換える。プレイリストがなければ false。
pub async fn update(
    pool: &Pool,
    id: &str,
    changes: &Changes,
    tracks: Option<&[String]>,
    now: i64,
) -> Result<bool, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let updated = sqlx::query!(
        "UPDATE playlist SET name = COALESCE(?, name), comment = COALESCE(?, comment),
             public = COALESCE(?, public), changed_at = ?
         WHERE id = ?",
        changes.name,
        changes.comment,
        changes.public,
        now,
        id
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if updated == 0 {
        return Ok(false);
    }
    if let Some(tracks) = tracks {
        sqlx::query!("DELETE FROM playlist_entry WHERE playlist_id = ?", id)
            .execute(&mut *tx)
            .await?;
        for (position, track_id) in tracks.iter().enumerate() {
            let position = i64::try_from(position).unwrap_or(i64::MAX);
            sqlx::query!(
                "INSERT INTO playlist_entry (playlist_id, position, track_id) VALUES (?, ?, ?)",
                id,
                position,
                track_id
            )
            .execute(&mut *tx)
            .await?;
        }
    }
    tx.commit().await?;
    Ok(true)
}

/// プレイリストがなければ false。
pub async fn delete(pool: &Pool, id: &str) -> Result<bool, sqlx::Error> {
    let deleted = sqlx::query!("DELETE FROM playlist WHERE id = ?", id)
        .execute(pool)
        .await?
        .rows_affected();
    Ok(deleted > 0)
}

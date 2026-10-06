//! Spotify 連携の接続と、お気に入りの曲の対応表（docs/spotify.md）。

use std::collections::HashSet;

use super::{Connection, Pool};

/// 対応の付け方。DB の spotify_track.match_method に入れる。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchMethod {
    Isrc,
    MatchKey,
    Fuzzy,
    Manual,
    /// 手動で対応を外した。自動の対応を付けない
    Ignored,
}

impl MatchMethod {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Isrc => "isrc",
            Self::MatchKey => "match_key",
            Self::Fuzzy => "fuzzy",
            Self::Manual => "manual",
            Self::Ignored => "ignored",
        }
    }
}

/// Spotify から読んだお気に入りの曲。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpotifyTrack {
    pub spotify_id: String,
    pub isrc: Option<String>,
    pub title: String,
    pub artists: Vec<String>,
    pub album: String,
    pub disc_number: Option<i64>,
    pub track_number: Option<i64>,
    pub duration_ms: i64,
    /// Spotify でお気に入りにした日時
    pub added_at: i64,
}

/// 対応表の一行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpotifyTrackRow {
    pub track: SpotifyTrack,
    pub track_id: Option<String>,
    pub match_method: Option<String>,
}

/// 照合に使うローカルの曲。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalTrack {
    pub id: String,
    pub title: String,
    pub match_key: String,
    /// アーティストの `match_key`。曲での並び順
    pub artist_keys: Vec<String>,
    /// 配信に使うファイルの長さ
    pub duration_ms: i64,
    /// 曲のファイルのタグにある ISRC
    pub isrcs: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Counts {
    pub total: i64,
    pub matched: i64,
}

/// 保存したリフレッシュトークンと、最後に一覧を読み始めた日時。接続していなければ None。
pub async fn account(pool: &Pool) -> Result<Option<(String, Option<i64>)>, sqlx::Error> {
    let row = sqlx::query!("SELECT refresh_token, fetched_at FROM spotify_account WHERE id = 1")
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|r| (r.refresh_token, r.fetched_at)))
}

/// 接続する。前の接続があれば置き換える。
pub async fn connect(pool: &Pool, refresh_token: &str, now: i64) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO spotify_account (id, refresh_token, connected_at) VALUES (1, ?, ?)
         ON CONFLICT (id) DO UPDATE SET refresh_token = excluded.refresh_token,
             connected_at = excluded.connected_at, fetched_at = NULL",
        refresh_token,
        now
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// リフレッシュで新しいトークンが返ったときに置き換える。
pub async fn update_refresh_token(pool: &Pool, refresh_token: &str) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE spotify_account SET refresh_token = ? WHERE id = 1",
        refresh_token
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn set_fetched_at(pool: &Pool, at: i64) -> Result<(), sqlx::Error> {
    sqlx::query!("UPDATE spotify_account SET fetched_at = ? WHERE id = 1", at)
        .execute(pool)
        .await?;
    Ok(())
}

/// 接続を切る。対応表は残す。
/// 接続を切る。接続していたら true。
pub async fn disconnect(pool: &Pool) -> Result<bool, sqlx::Error> {
    let rows = sqlx::query!("DELETE FROM spotify_account")
        .execute(pool)
        .await?
        .rows_affected();
    Ok(rows > 0)
}

/// Spotify から読んだ一覧で対応表を置き換える。一覧にない行は消し、ある行は対応を残して曲の情報だけ更新する。
pub async fn replace_tracks(pool: &Pool, tracks: &[SpotifyTrack]) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    let keep: HashSet<&str> = tracks.iter().map(|t| t.spotify_id.as_str()).collect();
    let stored = sqlx::query_scalar!(r#"SELECT spotify_id AS "id!" FROM spotify_track"#)
        .fetch_all(&mut *tx)
        .await?;
    for id in stored.iter().filter(|id| !keep.contains(id.as_str())) {
        sqlx::query!("DELETE FROM spotify_track WHERE spotify_id = ?", id)
            .execute(&mut *tx)
            .await?;
    }
    for t in tracks {
        let artists = serde_json::to_string(&t.artists).expect("文字列の配列は JSON にできる");
        sqlx::query!(
            "INSERT INTO spotify_track (spotify_id, isrc, title, artists, album, disc_number,
                 track_number, duration_ms, added_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT (spotify_id) DO UPDATE SET isrc = excluded.isrc, title = excluded.title,
                 artists = excluded.artists, album = excluded.album,
                 disc_number = excluded.disc_number, track_number = excluded.track_number,
                 duration_ms = excluded.duration_ms, added_at = excluded.added_at",
            t.spotify_id,
            t.isrc,
            t.title,
            artists,
            t.album,
            t.disc_number,
            t.track_number,
            t.duration_ms,
            t.added_at
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await
}

/// 対応表の行。`matched` が Some なら、対応の有無で絞る。お気に入りにした新しい順。
pub async fn tracks(
    pool: &Pool,
    matched: Option<bool>,
) -> Result<Vec<SpotifyTrackRow>, sqlx::Error> {
    let rows = sqlx::query!(
        r#"SELECT spotify_id AS "spotify_id!", isrc, title, artists, album, disc_number,
                  track_number, duration_ms, added_at, track_id, match_method
           FROM spotify_track
           WHERE ? IS NULL OR (track_id IS NOT NULL) = ?
           ORDER BY added_at DESC, spotify_id"#,
        matched,
        matched
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| SpotifyTrackRow {
            track: SpotifyTrack {
                spotify_id: r.spotify_id,
                isrc: r.isrc,
                title: r.title,
                artists: serde_json::from_str(&r.artists).unwrap_or_default(),
                album: r.album,
                disc_number: r.disc_number,
                track_number: r.track_number,
                duration_ms: r.duration_ms,
                added_at: r.added_at,
            },
            track_id: r.track_id,
            match_method: r.match_method,
        })
        .collect())
}

/// 自動で対応を付ける行。対応がなく、手動で外していないもの。
pub async fn unmatched(pool: &Pool) -> Result<Vec<SpotifyTrack>, sqlx::Error> {
    Ok(tracks(pool, Some(false))
        .await?
        .into_iter()
        .filter(|row| row.match_method.as_deref() != Some(MatchMethod::Ignored.as_str()))
        .map(|row| row.track)
        .collect())
}

pub async fn counts(pool: &Pool) -> Result<Counts, sqlx::Error> {
    let row = sqlx::query!(
        r#"SELECT COUNT(*) AS "total!: i64", COUNT(track_id) AS "matched!: i64"
           FROM spotify_track"#
    )
    .fetch_one(pool)
    .await?;
    Ok(Counts {
        total: row.total,
        matched: row.matched,
    })
}

/// 対応を付け、ローカルの曲をお気に入りにする。すでにお気に入りなら日時は古いほうにする。
/// `track_id` が None なら対応を外す。行がないか、曲がなければ false。
pub async fn link(
    pool: &Pool,
    spotify_id: &str,
    track_id: Option<&str>,
    method: MatchMethod,
) -> Result<bool, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let linked = link_in(&mut tx, spotify_id, track_id, method).await?;
    if linked {
        tx.commit().await?;
    }
    Ok(linked)
}

/// 自動で見つけた対応をまとめて付ける。付けた数を返す。
/// 一つのトランザクションにまとめる。コミットのたびにディスクへ書き切るので、一曲ずつだと遅いため。
pub async fn link_many(
    pool: &Pool,
    links: &[(&str, &str, MatchMethod)],
) -> Result<usize, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let mut linked = 0;
    for &(spotify_id, track_id, method) in links {
        if link_in(&mut tx, spotify_id, Some(track_id), method).await? {
            linked += 1;
        }
    }
    tx.commit().await?;
    Ok(linked)
}

async fn link_in(
    tx: &mut Connection,
    spotify_id: &str,
    track_id: Option<&str>,
    method: MatchMethod,
) -> Result<bool, sqlx::Error> {
    let Some(added_at) = sqlx::query_scalar!(
        "SELECT added_at FROM spotify_track WHERE spotify_id = ?",
        spotify_id
    )
    .fetch_optional(&mut *tx)
    .await?
    else {
        return Ok(false);
    };
    if let Some(track_id) = track_id {
        let starred = sqlx::query!(
            "UPDATE track SET starred_at = CASE WHEN starred_at IS NULL OR starred_at > ? THEN ?
                                                 ELSE starred_at END
             WHERE id = ?",
            added_at,
            added_at,
            track_id
        )
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if starred == 0 {
            return Ok(false);
        }
    }
    let method = method.as_str();
    sqlx::query!(
        "UPDATE spotify_track SET track_id = ?, match_method = ? WHERE spotify_id = ?",
        track_id,
        method,
        spotify_id
    )
    .execute(&mut *tx)
    .await?;
    Ok(true)
}

/// 照合に使うローカルの全曲。
pub async fn local_tracks(pool: &Pool) -> Result<Vec<LocalTrack>, sqlx::Error> {
    let rows = sqlx::query!(
        r#"SELECT track.id AS "id!", track.title, track.match_key, file.duration_ms
           FROM track JOIN file ON file.id = track.primary_file_id
           ORDER BY track.id"#
    )
    .fetch_all(pool)
    .await?;
    let mut tracks: Vec<LocalTrack> = rows
        .into_iter()
        .map(|r| LocalTrack {
            id: r.id,
            title: r.title,
            match_key: r.match_key,
            artist_keys: Vec::new(),
            duration_ms: r.duration_ms,
            isrcs: Vec::new(),
        })
        .collect();
    let index: std::collections::HashMap<String, usize> = tracks
        .iter()
        .enumerate()
        .map(|(i, t)| (t.id.clone(), i))
        .collect();

    let artists = sqlx::query!(
        r#"SELECT track_artist.track_id AS "track_id!", artist.match_key
           FROM track_artist JOIN artist ON artist.id = track_artist.artist_id
           ORDER BY track_artist.track_id, track_artist.position"#
    )
    .fetch_all(pool)
    .await?;
    for a in artists {
        if let Some(&i) = index.get(&a.track_id) {
            tracks[i].artist_keys.push(a.match_key);
        }
    }

    // ISRC は列に持たず、保存した生のタグから読む。スキャンの書き込みを変えずに済むため
    let files = sqlx::query!(r#"SELECT track_id AS "track_id!", tags FROM file ORDER BY path"#)
        .fetch_all(pool)
        .await?;
    for f in files {
        let Some(isrc) = crate::tags::from_stored(&f.tags).and_then(|(tags, _)| tags.isrc) else {
            continue;
        };
        if let Some(&i) = index.get(&f.track_id)
            && !tracks[i].isrcs.contains(&isrc)
        {
            tracks[i].isrcs.push(isrc);
        }
    }
    Ok(tracks)
}

//! 読みの手動設定（docs/schema.md の「日本語の並べ替え」）。
//! 直したら、並べ替えのキーと検索の文字列もその場で作り直す。

use super::{Connection, Pool};
use crate::tags::{ReadingSource, album_text, artist_text, sort_key, track_text};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Artist,
    Album,
    Track,
}

impl Kind {
    /// ID の接頭辞から種類を決める。
    pub fn of(id: &str) -> Option<Self> {
        match id.split_once('-')?.0 {
            "ar" => Some(Self::Artist),
            "al" => Some(Self::Album),
            "tr" => Some(Self::Track),
            _ => None,
        }
    }
}

/// 読みを直す対象の一行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub id: String,
    pub name: String,
    /// アルバムと曲の表示用アーティスト。アーティストでは None
    pub artist: Option<String>,
    /// 手動の読みがあればそれ、なければ自動の読み
    pub reading: Option<String>,
    pub source: Option<String>,
    /// スキャンで得た自動の読み。手動の読みを外すとこれに戻る
    pub auto_reading: Option<String>,
    pub auto_source: Option<String>,
}

/// 一覧の絞り込み。`None` は読みのないもの、`Source` はその出どころ（`sort_name_source` の値）のもの。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filter<'a> {
    All,
    None,
    Source(&'a str),
}

/// 種類ごとに、読みの順で一覧を返す。
pub async fn list(
    pool: &Pool,
    kind: Kind,
    filter: Filter<'_>,
    offset: i64,
    limit: i64,
) -> Result<Vec<Entry>, sqlx::Error> {
    let (all, none, source) = match filter {
        Filter::All => (true, false, ""),
        Filter::None => (false, true, ""),
        Filter::Source(source) => (false, false, source),
    };
    let entries = match kind {
        Kind::Artist => sqlx::query!(
            r#"SELECT id AS "id!", name, sort_name, sort_name_source, auto_sort_name,
                      auto_sort_name_source
               FROM artist
               WHERE ? OR (? AND sort_name IS NULL) OR sort_name_source = ?
               ORDER BY sort_key, id LIMIT ? OFFSET ?"#,
            all,
            none,
            source,
            limit,
            offset
        )
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|r| Entry {
            id: r.id,
            name: r.name,
            artist: None,
            reading: r.sort_name,
            source: r.sort_name_source,
            auto_reading: r.auto_sort_name,
            auto_source: r.auto_sort_name_source,
        })
        .collect(),
        Kind::Album => sqlx::query!(
            r#"SELECT id AS "id!", name, display_artist, sort_name, sort_name_source,
                      auto_sort_name, auto_sort_name_source
               FROM album
               WHERE ? OR (? AND sort_name IS NULL) OR sort_name_source = ?
               ORDER BY sort_key, id LIMIT ? OFFSET ?"#,
            all,
            none,
            source,
            limit,
            offset
        )
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|r| Entry {
            id: r.id,
            name: r.name,
            artist: Some(r.display_artist),
            reading: r.sort_name,
            source: r.sort_name_source,
            auto_reading: r.auto_sort_name,
            auto_source: r.auto_sort_name_source,
        })
        .collect(),
        Kind::Track => sqlx::query!(
            r#"SELECT id AS "id!", title, display_artist, sort_name, sort_name_source,
                      auto_sort_name, auto_sort_name_source
               FROM track
               WHERE ? OR (? AND sort_name IS NULL) OR sort_name_source = ?
               ORDER BY sort_key, id LIMIT ? OFFSET ?"#,
            all,
            none,
            source,
            limit,
            offset
        )
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|r| Entry {
            id: r.id,
            name: r.title,
            artist: Some(r.display_artist),
            reading: r.sort_name,
            source: r.sort_name_source,
            auto_reading: r.auto_sort_name,
            auto_source: r.auto_sort_name_source,
        })
        .collect(),
    };
    Ok(entries)
}

/// 手動の読みを設定する。`reading` が None なら手動の読みを外し、自動の読みに戻す。
/// 知らない ID なら None。`reading` は [`crate::tags::manual_reading`] でそろえたものを渡す。
pub async fn set(
    pool: &Pool,
    id: &str,
    reading: Option<&str>,
) -> Result<Option<Entry>, sqlx::Error> {
    let Some(kind) = Kind::of(id) else {
        return Ok(None);
    };
    let manual = ReadingSource::Manual.as_str();
    let mut tx = pool.begin().await?;
    match kind {
        Kind::Artist => {
            let Some(row) = sqlx::query!(
                "SELECT name, auto_sort_name, auto_sort_name_source FROM artist WHERE id = ?",
                id
            )
            .fetch_optional(&mut *tx)
            .await?
            else {
                return Ok(None);
            };
            let (sort_name, source) = match reading {
                Some(reading) => (Some(reading), Some(manual)),
                None => (
                    row.auto_sort_name.as_deref(),
                    row.auto_sort_name_source.as_deref(),
                ),
            };
            let manual_name = reading.map(|_| row.name.as_str());
            let key = sort_key(&row.name, sort_name);
            let text = artist_text(&row.name, sort_name);
            sqlx::query!(
                "UPDATE artist SET manual_sort_name = ?, manual_name = ?, sort_name = ?,
                     sort_name_source = ?, sort_key = ?, search_text = ?
                 WHERE id = ?",
                reading,
                manual_name,
                sort_name,
                source,
                key,
                text,
                id
            )
            .execute(&mut *tx)
            .await?;
            // アーティストの読みでも曲とアルバムを探せるので、その文字列も作り直す
            let tracks = sqlx::query_scalar!(
                "SELECT DISTINCT track_id FROM track_artist WHERE artist_id = ?",
                id
            )
            .fetch_all(&mut *tx)
            .await?;
            for track in tracks {
                refresh_track_text(&mut tx, &track).await?;
            }
            let albums = sqlx::query_scalar!(
                "SELECT DISTINCT album_id FROM album_artist WHERE artist_id = ?",
                id
            )
            .fetch_all(&mut *tx)
            .await?;
            for album in albums {
                refresh_album_text(&mut tx, &album).await?;
            }
        }
        Kind::Album => {
            let Some(row) = sqlx::query!(
                "SELECT name, auto_sort_name, auto_sort_name_source FROM album WHERE id = ?",
                id
            )
            .fetch_optional(&mut *tx)
            .await?
            else {
                return Ok(None);
            };
            let (sort_name, source) = match reading {
                Some(reading) => (Some(reading), Some(manual)),
                None => (
                    row.auto_sort_name.as_deref(),
                    row.auto_sort_name_source.as_deref(),
                ),
            };
            let manual_name = reading.map(|_| row.name.as_str());
            let key = sort_key(&row.name, sort_name);
            sqlx::query!(
                "UPDATE album SET manual_sort_name = ?, manual_name = ?, sort_name = ?,
                     sort_name_source = ?, sort_key = ?
                 WHERE id = ?",
                reading,
                manual_name,
                sort_name,
                source,
                key,
                id
            )
            .execute(&mut *tx)
            .await?;
            // 曲の検索の文字列はアルバム名だけを含み、アルバムの読みは含まないので作り直さない
            refresh_album_text(&mut tx, id).await?;
        }
        Kind::Track => {
            let Some(row) = sqlx::query!(
                "SELECT title, auto_sort_name, auto_sort_name_source FROM track WHERE id = ?",
                id
            )
            .fetch_optional(&mut *tx)
            .await?
            else {
                return Ok(None);
            };
            let (sort_name, source) = match reading {
                Some(reading) => (Some(reading), Some(manual)),
                None => (
                    row.auto_sort_name.as_deref(),
                    row.auto_sort_name_source.as_deref(),
                ),
            };
            let manual_name = reading.map(|_| row.title.as_str());
            let key = sort_key(&row.title, sort_name);
            sqlx::query!(
                "UPDATE track SET manual_sort_name = ?, manual_name = ?, sort_name = ?,
                     sort_name_source = ?, sort_key = ?
                 WHERE id = ?",
                reading,
                manual_name,
                sort_name,
                source,
                key,
                id
            )
            .execute(&mut *tx)
            .await?;
            refresh_track_text(&mut tx, id).await?;
        }
    }
    let entry = get(&mut tx, kind, id).await?;
    tx.commit().await?;
    Ok(entry)
}

async fn get(conn: &mut Connection, kind: Kind, id: &str) -> Result<Option<Entry>, sqlx::Error> {
    let entry = match kind {
        Kind::Artist => sqlx::query!(
            r#"SELECT id AS "id!", name, sort_name, sort_name_source, auto_sort_name,
                      auto_sort_name_source
               FROM artist WHERE id = ?"#,
            id
        )
        .fetch_optional(&mut *conn)
        .await?
        .map(|r| Entry {
            id: r.id,
            name: r.name,
            artist: None,
            reading: r.sort_name,
            source: r.sort_name_source,
            auto_reading: r.auto_sort_name,
            auto_source: r.auto_sort_name_source,
        }),
        Kind::Album => sqlx::query!(
            r#"SELECT id AS "id!", name, display_artist, sort_name, sort_name_source,
                      auto_sort_name, auto_sort_name_source
               FROM album WHERE id = ?"#,
            id
        )
        .fetch_optional(&mut *conn)
        .await?
        .map(|r| Entry {
            id: r.id,
            name: r.name,
            artist: Some(r.display_artist),
            reading: r.sort_name,
            source: r.sort_name_source,
            auto_reading: r.auto_sort_name,
            auto_source: r.auto_sort_name_source,
        }),
        Kind::Track => sqlx::query!(
            r#"SELECT id AS "id!", title, display_artist, sort_name, sort_name_source,
                      auto_sort_name, auto_sort_name_source
               FROM track WHERE id = ?"#,
            id
        )
        .fetch_optional(&mut *conn)
        .await?
        .map(|r| Entry {
            id: r.id,
            name: r.title,
            artist: Some(r.display_artist),
            reading: r.sort_name,
            source: r.sort_name_source,
            auto_reading: r.auto_sort_name,
            auto_source: r.auto_sort_name_source,
        }),
    };
    Ok(entry)
}

/// 曲の検索の文字列を、スキャンと同じ欄から作り直す。
async fn refresh_track_text(conn: &mut Connection, id: &str) -> Result<(), sqlx::Error> {
    let Some(track) = sqlx::query!(
        "SELECT t.title, t.sort_name, t.display_artist, al.name AS album
         FROM track t JOIN album al ON al.id = t.album_id WHERE t.id = ?",
        id
    )
    .fetch_optional(&mut *conn)
    .await?
    else {
        return Ok(());
    };
    let readings = sqlx::query_scalar!(
        r#"SELECT a.sort_name AS "sort_name!" FROM track_artist ta
           JOIN artist a ON a.id = ta.artist_id
           WHERE ta.track_id = ? AND a.sort_name IS NOT NULL ORDER BY ta.position"#,
        id
    )
    .fetch_all(&mut *conn)
    .await?;
    let text = track_text(
        &track.title,
        track.sort_name.as_deref(),
        &track.display_artist,
        &track.album,
        readings.iter().map(String::as_str),
    );
    sqlx::query!("UPDATE track SET search_text = ? WHERE id = ?", text, id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// アルバムの検索の文字列を、スキャンと同じ欄から作り直す。
async fn refresh_album_text(conn: &mut Connection, id: &str) -> Result<(), sqlx::Error> {
    let Some(album) = sqlx::query!(
        "SELECT name, sort_name, display_artist FROM album WHERE id = ?",
        id
    )
    .fetch_optional(&mut *conn)
    .await?
    else {
        return Ok(());
    };
    let readings = sqlx::query_scalar!(
        r#"SELECT a.sort_name AS "sort_name!" FROM album_artist aa
           JOIN artist a ON a.id = aa.artist_id
           WHERE aa.album_id = ? AND a.sort_name IS NOT NULL ORDER BY aa.position"#,
        id
    )
    .fetch_all(&mut *conn)
    .await?;
    let text = album_text(
        &album.name,
        album.sort_name.as_deref(),
        &album.display_artist,
        readings.iter().map(String::as_str),
    );
    sqlx::query!("UPDATE album SET search_text = ? WHERE id = ?", text, id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

//! 検索の問い合わせ。方式を替えやすいよう、このモジュールに閉じ込める（docs/server.md）。
//!
//! 今は `search_text` 列への `LIKE` の部分一致で、全行を順に見る。
//! 語の数が決まらないので、ここだけは実行時に SQL を組み立て、コンパイル時の照合は使わない。

use sqlx::{QueryBuilder, Sqlite};

use super::Pool;
use super::browse::ArtistSummary;

/// 名前か読みにすべての語を含むアーティスト。曲にだけ参加しているアーティストも含める。
pub async fn artists(
    pool: &Pool,
    words: &[String],
    count: i64,
    offset: i64,
) -> Result<Vec<ArtistSummary>, sqlx::Error> {
    let mut query = QueryBuilder::<Sqlite>::new(
        "SELECT artist.id, artist.name, artist.sort_name,
                (SELECT COUNT(*) FROM (
                    SELECT album_id FROM album_artist WHERE artist_id = artist.id
                    UNION
                    SELECT track.album_id FROM track
                      JOIN track_artist ON track_artist.track_id = track.id
                      WHERE track_artist.artist_id = artist.id
                    UNION
                    SELECT track.album_id FROM track
                      JOIN track_contributor ON track_contributor.track_id = track.id
                      WHERE track_contributor.artist_id = artist.id)) AS album_count,
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
                      THEN 'arranger ' ELSE '' END) AS roles,
                artist.starred_at, artist.rating
         FROM artist",
    );
    push_filter(&mut query, "artist", words);
    push_page(&mut query, "artist", count, offset);
    query.build_query_as().fetch_all(pool).await
}

/// 名前、読み、表示用アーティストにすべての語を含むアルバムの ID。
pub async fn albums(
    pool: &Pool,
    words: &[String],
    count: i64,
    offset: i64,
) -> Result<Vec<String>, sqlx::Error> {
    let mut query = QueryBuilder::<Sqlite>::new("SELECT album.id FROM album");
    push_filter(&mut query, "album", words);
    push_page(&mut query, "album", count, offset);
    query.build_query_scalar().fetch_all(pool).await
}

/// 曲の結果の並び順（docs/schema.md の「検索」）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SongSort {
    /// 曲名の読みの順
    #[default]
    Title,
    /// アルバム名の読みの順。アルバムの中はディスク番号、トラック番号の順
    Album,
    /// 曲のアーティストの一人目の読みの順。同じアーティストの中はアルバムの順
    Artist,
}

/// アルバムの順。ID で順序を確定させる。
const ALBUM_ORDER: &str = "album.sort_key, album.id,
    track.disc_number IS NULL, track.disc_number,
    track.track_number IS NULL, track.track_number, track.sort_key, track.id";

/// 曲のアーティストの一人目の並べ替えキー。
const FIRST_ARTIST: &str = "(SELECT artist.sort_key FROM track_artist
    JOIN artist ON artist.id = track_artist.artist_id
    WHERE track_artist.track_id = track.id
    ORDER BY track_artist.position LIMIT 1)";

/// 曲名、読み、表示用アーティスト、アルバム名にすべての語を含む曲の ID。
pub async fn songs(
    pool: &Pool,
    words: &[String],
    sort: SongSort,
    count: i64,
    offset: i64,
) -> Result<Vec<String>, sqlx::Error> {
    let mut query = QueryBuilder::<Sqlite>::new(
        "SELECT track.id FROM track JOIN album ON album.id = track.album_id",
    );
    push_filter(&mut query, "track", words);
    match sort {
        SongSort::Title => query.push(" ORDER BY track.sort_key, track.id"),
        SongSort::Album => query.push(format!(" ORDER BY {ALBUM_ORDER}")),
        SongSort::Artist => query.push(format!(
            " ORDER BY {FIRST_ARTIST} IS NULL, {FIRST_ARTIST}, {ALBUM_ORDER}"
        )),
    };
    push_limit(&mut query, count, offset);
    query.build_query_scalar().fetch_all(pool).await
}

/// 語ごとに `search_text LIKE '%語%'` を AND でつなぐ。語がなければ全件。
fn push_filter(query: &mut QueryBuilder<Sqlite>, table: &str, words: &[String]) {
    for (i, word) in words.iter().enumerate() {
        query.push(if i == 0 { " WHERE " } else { " AND " });
        query.push(table);
        query.push(".search_text LIKE ");
        query.push_bind(like_pattern(word));
        query.push(" ESCAPE '\\'");
    }
}

/// 並べ替えキーの順。ID で順序を確定させ、offset で続きを取っても抜けや重なりが出ないようにする。
fn push_page(query: &mut QueryBuilder<Sqlite>, table: &str, count: i64, offset: i64) {
    query.push(format!(" ORDER BY {table}.sort_key, {table}.id"));
    push_limit(query, count, offset);
}

fn push_limit(query: &mut QueryBuilder<Sqlite>, count: i64, offset: i64) {
    query.push(" LIMIT ");
    query.push_bind(count);
    query.push(" OFFSET ");
    query.push_bind(offset);
}

/// `%` と `_` と `\` を文字として扱う部分一致の形。
fn like_pattern(word: &str) -> String {
    let mut pattern = String::with_capacity(word.len() + 2);
    pattern.push('%');
    for c in word.chars() {
        if matches!(c, '%' | '_' | '\\') {
            pattern.push('\\');
        }
        pattern.push(c);
    }
    pattern.push('%');
    pattern
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn like_pattern_escapes_wildcards() {
        assert_eq!(like_pattern("100%_a\\b"), "%100\\%\\_a\\\\b%");
    }
}

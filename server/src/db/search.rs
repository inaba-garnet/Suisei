//! 検索の問い合わせ。方式を替えやすいよう、このモジュールに閉じ込める（docs/server.md）。
//!
//! 今は `search_text` 列への `LIKE` の部分一致で、全行を順に見る。
//! 結果は名前との一致の度合い（[`push_rank`]）で段に分け、段の中を読みの順に並べる。
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
    // アーティストの照らす文字列は名前と読みだけなので、そのまま一致の度合いに使う
    push_page(
        &mut query,
        "artist",
        "artist.search_text",
        words,
        count,
        offset,
    );
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
    push_page(&mut query, "album", "album.name_text", words, count, offset);
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
    query.push(" ORDER BY ");
    push_rank(&mut query, "track.name_text", words);
    query.push(song_order(sort));
    push_limit(&mut query, count, offset);
    query.build_query_scalar().fetch_all(pool).await
}

/// `genre`（表記ゆれをまとめたジャンル名）の曲の ID。
pub async fn songs_by_genre(
    pool: &Pool,
    genre: &str,
    sort: SongSort,
    count: i64,
    offset: i64,
) -> Result<Vec<String>, sqlx::Error> {
    let mut query = QueryBuilder::<Sqlite>::new(
        "SELECT track.id FROM track JOIN album ON album.id = track.album_id
         WHERE EXISTS (SELECT 1 FROM track_genre
                       WHERE track_genre.track_id = track.id AND track_genre.genre = ",
    );
    query.push_bind(genre);
    query.push(")");
    push_song_order(&mut query, sort);
    push_limit(&mut query, count, offset);
    query.build_query_scalar().fetch_all(pool).await
}

/// お気に入りの曲の ID を `sort` の順に返す。
pub async fn starred_songs(pool: &Pool, sort: SongSort) -> Result<Vec<String>, sqlx::Error> {
    let mut query = QueryBuilder::<Sqlite>::new(
        "SELECT track.id FROM track JOIN album ON album.id = track.album_id
         WHERE track.starred_at IS NOT NULL",
    );
    push_song_order(&mut query, sort);
    query.build_query_scalar().fetch_all(pool).await
}

/// `songs` の全件と同じ順で、各曲の見出しを決めるキーを返す（docs/web.md の「一覧」）。
/// 曲名順は曲の、アルバム順はアルバムの、アーティスト順は曲のアーティストの一人目の並べ替えキー。アーティストがいなければ `None`。
pub async fn song_heading_keys(
    pool: &Pool,
    sort: SongSort,
) -> Result<Vec<Option<String>>, sqlx::Error> {
    let key = match sort {
        SongSort::Title => "track.sort_key",
        SongSort::Album => "album.sort_key",
        SongSort::Artist => FIRST_ARTIST,
    };
    let mut query = QueryBuilder::<Sqlite>::new(format!(
        "SELECT {key} FROM track JOIN album ON album.id = track.album_id"
    ));
    push_song_order(&mut query, sort);
    query.build_query_scalar().fetch_all(pool).await
}

fn push_song_order(query: &mut QueryBuilder<Sqlite>, sort: SongSort) {
    query.push(" ORDER BY ");
    query.push(song_order(sort));
}

fn song_order(sort: SongSort) -> String {
    match sort {
        SongSort::Title => "track.sort_key, track.id".to_owned(),
        SongSort::Album => ALBUM_ORDER.to_owned(),
        SongSort::Artist => format!("{FIRST_ARTIST} IS NULL, {FIRST_ARTIST}, {ALBUM_ORDER}"),
    }
}

/// 名前との一致の度合いで段に分ける並べ替えのキーを、後ろに `, ` を付けて足す。語がなければ足さない。
/// 名前と読みの欄（`name`）のどれかが検索語と同じなら 0、検索語で始まれば 1、すべての語を含めば 2、
/// 名前の外（アーティストやアルバム）だけで当たれば 3。検索語は語を空白一つでつないだもの。
fn push_rank(query: &mut QueryBuilder<Sqlite>, name: &str, words: &[String]) {
    if words.is_empty() {
        return;
    }
    let phrase = escape_like(&words.join(" "));
    query.push(format!("CASE WHEN char(31) || {name} || char(31) LIKE "));
    query.push_bind(format!("%{SEP}{phrase}{SEP}%"));
    query.push(format!(" ESCAPE '\\' THEN 0 WHEN char(31) || {name} LIKE "));
    query.push_bind(format!("%{SEP}{phrase}%"));
    query.push(" ESCAPE '\\' THEN 1 WHEN ");
    for (i, word) in words.iter().enumerate() {
        if i > 0 {
            query.push(" AND ");
        }
        query.push(format!("{name} LIKE "));
        query.push_bind(like_pattern(word));
        query.push(" ESCAPE '\\'");
    }
    query.push(" THEN 2 ELSE 3 END, ");
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

/// 一致の度合い、並べ替えキーの順。ID で順序を確定させ、offset で続きを取っても抜けや重なりが出ないようにする。
fn push_page(
    query: &mut QueryBuilder<Sqlite>,
    table: &str,
    name: &str,
    words: &[String],
    count: i64,
    offset: i64,
) {
    query.push(" ORDER BY ");
    push_rank(query, name, words);
    query.push(format!("{table}.sort_key, {table}.id"));
    push_limit(query, count, offset);
}

fn push_limit(query: &mut QueryBuilder<Sqlite>, count: i64, offset: i64) {
    query.push(" LIMIT ");
    query.push_bind(count);
    query.push(" OFFSET ");
    query.push_bind(offset);
}

/// 照らす文字列の欄の区切り（`tags::search_text`）。SQL では `char(31)`
const SEP: char = '\u{1f}';

/// `%` と `_` と `\` を文字として扱う部分一致の形。
fn like_pattern(word: &str) -> String {
    format!("%{}%", escape_like(word))
}

/// `%` と `_` と `\` の前に `\` を置き、`LIKE` で文字として扱わせる。
fn escape_like(word: &str) -> String {
    let mut escaped = String::with_capacity(word.len());
    for c in word.chars() {
        if matches!(c, '%' | '_' | '\\') {
            escaped.push('\\');
        }
        escaped.push(c);
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn like_pattern_escapes_wildcards() {
        assert_eq!(like_pattern("100%_a\\b"), "%100\\%\\_a\\\\b%");
    }
}

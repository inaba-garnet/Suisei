//! 閲覧のエンドポイント。

use std::collections::HashMap;
use std::time::{Duration, UNIX_EPOCH};

use serde_json::{Map, Value, json};

use super::{AppState, payload};
use crate::db::browse::{self, Album, AlbumOrder, ArtistEntry, ArtistSummary, Credit, Song};
use crate::subsonic::{Error, ErrorCode, Params};
use crate::tags::index_heading;

/// 閲覧のエンドポイントでなければ `None` を返す。
pub async fn respond(
    name: &str,
    params: &Params,
    state: &AppState,
) -> Option<Result<Map<String, Value>, Error>> {
    let result = match name {
        "getArtists" => artists(params, state).await,
        "getIndexes" => indexes(params, state).await,
        "getArtist" => artist(params, state).await,
        "getAlbum" => album(params, state).await,
        "getSong" => song(params, state).await,
        "getAlbumList2" => album_list(params, state).await,
        "getGenres" => genres(state).await,
        "search3" => super::search::search3(params, state).await,
        _ => return None,
    };
    Some(result)
}

pub(super) fn db_error(err: sqlx::Error) -> Error {
    tracing::error!(error = %err, "db error");
    Error::new(ErrorCode::Generic, "database error")
}

fn not_found(what: &str) -> Error {
    Error::new(ErrorCode::NotFound, format!("{what} not found"))
}

/// `id` を読み、マージで消えた ID なら引き継いだ ID に引き直す。
async fn id_param(params: &Params, state: &AppState) -> Result<String, Error> {
    let id = params.get("id").ok_or_else(|| {
        Error::new(
            ErrorCode::MissingParameter,
            "required parameter is missing: id",
        )
    })?;
    browse::resolve_alias(&state.db, id).await.map_err(db_error)
}

/// 独自の引数 `role` で、その役割のアーティストを返す。既定はアルバムアーティスト（docs/schema.md の「日本語の並べ替え」）。
async fn artists(params: &Params, state: &AppState) -> Result<Map<String, Value>, Error> {
    let role = params.get("role").unwrap_or("albumartist");
    let entries = match role {
        "albumartist" => browse::album_artists(&state.db).await,
        "artist" => browse::track_artists(&state.db).await,
        "composer" | "lyricist" | "arranger" => browse::contributors(&state.db, role).await,
        _ => {
            return Err(Error::new(
                ErrorCode::Generic,
                format!("unknown role: {role}"),
            ));
        }
    }
    .map_err(db_error)?;
    let index = group(&entries, |a| {
        let mut artist = json!({
            "id": a.id,
            "name": a.name,
            "albumCount": a.album_count,
            "roles": browse::roles(&a.roles),
        });
        if let Some(sort_name) = &a.sort_name {
            artist["sortName"] = json!(sort_name);
        }
        annotate(&mut artist, a.starred_at, a.rating);
        artist
    });
    Ok(payload(json!({ "artists": {
        "ignoredArticles": "",
        "lastModified": last_modified(state),
        "index": index,
    }})))
}

/// 音楽フォルダが一つなので、getArtists と同じアーティストを返す。
/// `ifModifiedSince` が最後のスキャンより後なら、変わっていないので中身を返さない（Subsonic の仕様）。
async fn indexes(params: &Params, state: &AppState) -> Result<Map<String, Value>, Error> {
    let last_modified = last_modified(state);
    let unchanged = params
        .get("ifModifiedSince")
        .and_then(|v| v.parse::<i64>().ok())
        .is_some_and(|since| since >= last_modified);
    let mut indexes = json!({ "ignoredArticles": "", "lastModified": last_modified });
    if !unchanged {
        let entries = browse::album_artists(&state.db).await.map_err(db_error)?;
        indexes["index"] = json!(group(&entries, |a| {
            let mut artist = json!({ "id": a.id, "name": a.name });
            annotate(&mut artist, a.starred_at, a.rating);
            artist
        }));
    }
    Ok(payload(json!({ "indexes": indexes })))
}

/// 並べ替えキーの順に並んだアーティストを、見出しごとにまとめる。
fn group(entries: &[ArtistEntry], artist: impl Fn(&ArtistEntry) -> Value) -> Vec<Value> {
    let mut index: Vec<(String, Vec<Value>)> = Vec::new();
    for entry in entries {
        let heading = index_heading(&entry.sort_key);
        match index.last_mut() {
            Some((last, artists)) if *last == heading => artists.push(artist(entry)),
            _ => index.push((heading, vec![artist(entry)])),
        }
    }
    index
        .into_iter()
        .map(|(name, artists)| json!({ "name": name, "artist": artists }))
        .collect()
}

/// 最後にスキャンした時刻（UNIX 時刻のミリ秒）。まだなら 0。
fn last_modified(state: &AppState) -> i64 {
    state
        .scanner
        .status()
        .last_scan
        .and_then(|at| at.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

async fn artist(params: &Params, state: &AppState) -> Result<Map<String, Value>, Error> {
    let id = id_param(params, state).await?;
    let artist = browse::artist(&state.db, &id)
        .await
        .map_err(db_error)?
        .ok_or_else(|| not_found("artist"))?;
    let albums = browse::albums_of_artist(&state.db, &id)
        .await
        .map_err(db_error)?;
    let mut list = Vec::with_capacity(albums.len());
    for album in &albums {
        list.push(album_json(state, album).await?);
    }
    let mut value = json!({
        "id": artist.id,
        "name": artist.name,
        "albumCount": albums.len(),
        "roles": browse::roles(&artist.roles),
        "album": list,
    });
    if let Some(sort_name) = artist.sort_name {
        value["sortName"] = json!(sort_name);
    }
    annotate(&mut value, artist.starred_at, artist.rating);
    Ok(payload(json!({ "artist": value })))
}

async fn album(params: &Params, state: &AppState) -> Result<Map<String, Value>, Error> {
    let id = id_param(params, state).await?;
    let album = browse::album(&state.db, &id)
        .await
        .map_err(db_error)?
        .ok_or_else(|| not_found("album"))?;
    let mut value = album_json(state, &album).await?;
    let songs = browse::songs_of_album(&state.db, &id)
        .await
        .map_err(db_error)?;
    value["song"] = json!(songs_json(state, &album.id, &songs).await?);
    Ok(payload(json!({ "album": value })))
}

async fn song(params: &Params, state: &AppState) -> Result<Map<String, Value>, Error> {
    let id = id_param(params, state).await?;
    let song = browse::song(&state.db, &id)
        .await
        .map_err(db_error)?
        .ok_or_else(|| not_found("song"))?;
    let album_id = song.album_id.clone();
    let mut songs = songs_json(state, &album_id, &[song]).await?;
    Ok(payload(json!({ "song": songs.remove(0) })))
}

/// Subsonic の `size` の既定値と上限。
const LIST_SIZE_DEFAULT: i64 = 10;
const LIST_SIZE_MAX: i64 = 500;

async fn album_list(params: &Params, state: &AppState) -> Result<Map<String, Value>, Error> {
    let kind = params.get("type").ok_or_else(|| {
        Error::new(
            ErrorCode::MissingParameter,
            "required parameter is missing: type",
        )
    })?;
    let number = |key: &str| params.get(key).and_then(|v| v.parse::<i64>().ok());
    let required = |key: &str| {
        number(key).ok_or_else(|| {
            Error::new(
                ErrorCode::MissingParameter,
                format!("required parameter is missing: {key}"),
            )
        })
    };
    let order = match kind {
        "newest" => AlbumOrder::Newest,
        "alphabeticalByName" => AlbumOrder::ByName,
        "alphabeticalByArtist" => AlbumOrder::ByArtist,
        "random" => AlbumOrder::Random,
        "byYear" => AlbumOrder::ByYear {
            from: required("fromYear")?,
            to: required("toYear")?,
        },
        "byGenre" => {
            let genre = params.get("genre").ok_or_else(|| {
                Error::new(
                    ErrorCode::MissingParameter,
                    "required parameter is missing: genre",
                )
            })?;
            let name = browse::genre_name(&state.db, genre)
                .await
                .map_err(db_error)?;
            // 知らないジャンルなら、そのまま絞り込んで空の一覧を返す
            AlbumOrder::ByGenre(name.unwrap_or_else(|| genre.to_owned()))
        }
        "recent" => AlbumOrder::Recent,
        "frequent" => AlbumOrder::Frequent,
        "starred" => AlbumOrder::Starred,
        "highest" => AlbumOrder::Highest,
        _ => {
            return Err(Error::new(
                ErrorCode::Generic,
                format!("unknown type: {kind}"),
            ));
        }
    };
    let size = number("size")
        .unwrap_or(LIST_SIZE_DEFAULT)
        .clamp(0, LIST_SIZE_MAX);
    let offset = number("offset").unwrap_or(0).max(0);
    let albums = browse::album_list(&state.db, &order, size, offset)
        .await
        .map_err(db_error)?;
    let mut list = Vec::with_capacity(albums.len());
    for album in &albums {
        list.push(album_json(state, album).await?);
    }
    Ok(payload(json!({ "albumList2": { "album": list } })))
}

async fn genres(state: &AppState) -> Result<Map<String, Value>, Error> {
    let genres = browse::genres(&state.db).await.map_err(db_error)?;
    let list: Vec<Value> = genres
        .iter()
        .map(|g| json!({ "value": g.name, "songCount": g.song_count, "albumCount": g.album_count }))
        .collect();
    Ok(payload(json!({ "genres": { "genre": list } })))
}

/// AlbumID3。
pub(super) async fn album_json(state: &AppState, album: &Album) -> Result<Value, Error> {
    let artists = browse::credits_of_album(&state.db, &album.id)
        .await
        .map_err(db_error)?;
    let genres = browse::album_genres(&state.db, &album.id)
        .await
        .map_err(db_error)?;
    let mut value = json!({
        "id": album.id,
        "name": album.name,
        "artist": album.display_artist,
        "displayArtist": album.display_artist,
        "songCount": album.song_count,
        "duration": seconds(album.duration_ms),
        "created": timestamp(album.created_at),
        "isCompilation": album.compilation,
        "playCount": album.play_count,
        "artists": artists_json(&artists),
        "genres": genres_json(&genres),
    });
    if let Some(first) = artists.first() {
        value["artistId"] = json!(first.artist_id);
    }
    // 画像のないアルバムに付けると、クライアントが取りに来ては失敗する
    if album.has_cover {
        value["coverArt"] = json!(album.id);
    }
    if let Some(year) = album.year {
        value["year"] = json!(year);
    }
    if let Some(at) = album.last_played {
        value["played"] = json!(timestamp(at));
    }
    annotate(&mut value, album.starred_at, album.rating);
    if let Some(genre) = genres.first() {
        value["genre"] = json!(genre);
    }
    if let Some(sort_name) = &album.sort_name {
        value["sortName"] = json!(sort_name);
    }
    Ok(value)
}

/// 同じアルバムの曲を Child にする。アーティストとジャンルはアルバムの分をまとめて読む。
pub(super) async fn songs_json(
    state: &AppState,
    album_id: &str,
    songs: &[Song],
) -> Result<Vec<Value>, Error> {
    let album_artists = browse::credits_of_album(&state.db, album_id)
        .await
        .map_err(db_error)?;
    let mut track_artists: HashMap<String, Vec<Credit>> = HashMap::new();
    for credit in browse::track_artists_of_album(&state.db, album_id)
        .await
        .map_err(db_error)?
    {
        track_artists
            .entry(credit.owner_id.clone())
            .or_default()
            .push(credit);
    }
    let mut contributors: HashMap<String, Vec<browse::Contributor>> = HashMap::new();
    for contributor in browse::contributors_of_album(&state.db, album_id)
        .await
        .map_err(db_error)?
    {
        contributors
            .entry(contributor.track_id.clone())
            .or_default()
            .push(contributor);
    }
    let mut track_genres: HashMap<String, Vec<String>> = HashMap::new();
    for (track_id, genre) in browse::track_genres_of_album(&state.db, album_id)
        .await
        .map_err(db_error)?
    {
        track_genres.entry(track_id).or_default().push(genre);
    }
    Ok(songs
        .iter()
        .map(|song| {
            let artists = track_artists.get(&song.id).map_or(&[][..], Vec::as_slice);
            let genres = track_genres.get(&song.id).map_or(&[][..], Vec::as_slice);
            let contributors = contributors.get(&song.id).map_or(&[][..], Vec::as_slice);
            song_json(song, artists, &album_artists, genres, contributors)
        })
        .collect())
}

/// Child。`path` は音楽フォルダからの相対パスで、サーバーの絶対パスは出さない。
fn song_json(
    song: &Song,
    artists: &[Credit],
    album_artists: &[Credit],
    genres: &[String],
    contributors: &[browse::Contributor],
) -> Value {
    let mut value = json!({
        "id": song.id,
        "parent": song.album_id,
        "isDir": false,
        "title": song.title,
        "album": song.album_name,
        "albumId": song.album_id,
        "artist": song.display_artist,
        "displayArtist": song.display_artist,
        "displayAlbumArtist": song.album_display_artist,
        "artists": artists_json(artists),
        "albumArtists": artists_json(album_artists),
        "genres": genres_json(genres),
        "size": song.size,
        "contentType": song.content_type,
        "suffix": song.suffix,
        "duration": seconds(song.duration_ms),
        "path": song.path,
        "created": timestamp(song.created_at),
        "playCount": song.play_count,
        "type": "music",
        "mediaType": "song",
    });
    let optional = [
        ("track", song.track_number),
        ("discNumber", song.disc_number),
        ("year", song.year),
        ("bitRate", song.bit_rate),
        ("samplingRate", song.sample_rate),
        ("channelCount", song.channels),
        ("bitDepth", song.bit_depth),
    ];
    for (key, v) in optional {
        if let Some(v) = v {
            value[key] = json!(v);
        }
    }
    if let Some(first) = artists.first() {
        value["artistId"] = json!(first.artist_id);
    }
    if let Some(at) = song.last_played {
        value["played"] = json!(timestamp(at));
    }
    annotate(&mut value, song.starred_at, song.rating);
    // 曲ごとの画像は扱わず、アルバムの画像を使う
    if song.album_has_cover {
        value["coverArt"] = json!(song.album_id);
    }
    if let Some(genre) = genres.first() {
        value["genre"] = json!(genre);
    }
    if let Some(sort_name) = &song.sort_name {
        value["sortName"] = json!(sort_name);
    }
    // OpenSubsonic では、対応する項目は値がなくても空で返す
    value["displayComposer"] = json!(song.display_composer.as_deref().unwrap_or_default());
    value["contributors"] = contributors
        .iter()
        .map(|c| json!({ "role": c.role, "artist": { "id": c.artist_id, "name": c.name } }))
        .collect();
    value
}

/// 検索とお気に入りの一覧に出す ArtistID3。
pub(super) fn artist_summary_json(a: &ArtistSummary) -> Value {
    let mut artist = json!({
        "id": a.id,
        "name": a.name,
        "albumCount": a.album_count,
        "roles": browse::roles(&a.roles),
    });
    if let Some(sort_name) = &a.sort_name {
        artist["sortName"] = json!(sort_name);
    }
    annotate(&mut artist, a.starred_at, a.rating);
    artist
}

/// お気に入りの日時と評価は、値があるときだけ付ける。
fn annotate(value: &mut Value, starred_at: Option<i64>, rating: Option<i64>) {
    if let Some(at) = starred_at {
        value["starred"] = json!(timestamp(at));
    }
    if let Some(rating) = rating {
        value["userRating"] = json!(rating);
    }
}

fn artists_json(credits: &[Credit]) -> Value {
    credits
        .iter()
        .map(|c| json!({ "id": c.artist_id, "name": c.name }))
        .collect()
}

fn genres_json(genres: &[String]) -> Value {
    genres.iter().map(|g| json!({ "name": g })).collect()
}

/// 秒に丸める。
pub(super) fn seconds(ms: i64) -> i64 {
    (ms + 500) / 1000
}

/// UNIX 時刻のミリ秒を ISO 8601 にする。
pub(super) fn timestamp(ms: i64) -> String {
    let at = UNIX_EPOCH + Duration::from_millis(u64::try_from(ms).unwrap_or(0));
    humantime::format_rfc3339_millis(at).to_string()
}

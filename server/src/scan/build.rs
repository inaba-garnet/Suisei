//! 読んだファイルから、曲、アルバム、アーティストを組み立てる。DB には触らない。

use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::path::Path;

use crate::db::library::{
    AlbumRow, ArtistRow, CreditRow, FileRow, GenreRow, Library, Snapshot, TrackRow,
};
use crate::db::{IdKind, new_id};
use crate::tags::{Credit, RawTags, TrackInfo, reading, sort_key};

/// スキャンで見つけたファイル。
#[derive(Debug, Clone)]
pub struct Scanned {
    /// `id` と `track_id` は、前回のスキャンで DB にあったときだけ入っている
    pub row: FileRow,
    pub tags: RawTags,
}

/// 同じ鍵にまとまるものと、前回それが属していた ID。
struct Group {
    key: String,
    /// 前回の ID。メンバーごとに一つ（なければ入れない）
    previous: Vec<String>,
}

/// ID を決める（docs/schema.md の「ID と同一判定」）。
/// 1. 鍵が既存の行と一致すれば、その ID
/// 2. 一致しなければ、メンバーが前回属していた ID のうち、まだ使われていないもので最も多いもの
/// 3. どれもなければ新しく採番する
///
/// 引き継がれなかった古い ID のメンバーが別の ID に移っていれば、その対応を別名として返す。
fn assign_ids(
    kind: IdKind,
    groups: &[Group],
    existing: &[(String, String)],
    taken: &mut HashSet<String>,
    aliases: &mut Vec<(String, String)>,
) -> Vec<String> {
    let by_key: HashMap<&str, &str> = existing
        .iter()
        .map(|(id, key)| (key.as_str(), id.as_str()))
        .collect();
    let mut ids: Vec<Option<String>> = groups
        .iter()
        .map(|g| by_key.get(g.key.as_str()).map(|id| (*id).to_owned()))
        .collect();
    let mut claimed: HashSet<String> = ids.iter().flatten().cloned().collect();
    let existing_ids: HashSet<&str> = existing.iter().map(|(id, _)| id.as_str()).collect();

    for (id, group) in ids.iter_mut().zip(groups) {
        if id.is_some() {
            continue;
        }
        let inherited = majority(
            group
                .previous
                .iter()
                .filter(|p| existing_ids.contains(p.as_str()) && !claimed.contains(*p)),
        )
        .cloned();
        let assigned = inherited.unwrap_or_else(|| fresh_id(kind, taken));
        claimed.insert(assigned.clone());
        *id = Some(assigned);
    }
    let ids: Vec<String> = ids.into_iter().flatten().collect();

    // 引き継がれなかった ID のメンバーがどこへ移ったか
    let mut moved: HashMap<&str, Vec<&str>> = HashMap::new();
    for (id, group) in ids.iter().zip(groups) {
        for previous in &group.previous {
            if !claimed.contains(previous) {
                moved.entry(previous).or_default().push(id);
            }
        }
    }
    for (old_id, _) in existing {
        if let Some(new_id) = moved
            .get(old_id.as_str())
            .and_then(|to| majority(to.iter()))
        {
            aliases.push((old_id.clone(), (*new_id).to_owned()));
        }
    }
    ids
}

/// 既存の ID と重ならない ID を採る。
fn fresh_id(kind: IdKind, taken: &mut HashSet<String>) -> String {
    loop {
        let id = new_id(kind);
        if taken.insert(id.clone()) {
            return id;
        }
    }
}

/// 最も多い値。同数なら先に現れたもの。
fn majority<T: Eq + Hash + Clone>(items: impl Iterator<Item = T>) -> Option<T> {
    let mut counts: Vec<(T, usize)> = Vec::new();
    let mut index: HashMap<T, usize> = HashMap::new();
    for item in items {
        match index.get(&item) {
            Some(&i) => counts[i].1 += 1,
            None => {
                index.insert(item.clone(), counts.len());
                counts.push((item, 1));
            }
        }
    }
    let max = counts.iter().map(|(_, n)| *n).max()?;
    counts.into_iter().find(|(_, n)| *n == max).map(|(v, _)| v)
}

/// 鍵ごとにまとめる。順序は最初に現れた順。
fn group_by<'a>(keys: impl Iterator<Item = &'a str>) -> (Vec<String>, Vec<usize>) {
    let mut order = Vec::new();
    let mut index: HashMap<&str, usize> = HashMap::new();
    let mut membership = Vec::new();
    for key in keys {
        let i = *index.entry(key).or_insert_with(|| {
            order.push(key.to_owned());
            order.len() - 1
        });
        membership.push(i);
    }
    (order, membership)
}

pub fn build(mut files: Vec<Scanned>, snapshot: &Snapshot, music_dir: &str, now: i64) -> Library {
    files.sort_by(|a, b| a.row.path.cmp(&b.row.path));
    let mut taken: HashSet<String> = snapshot
        .files
        .iter()
        .map(|f| f.id.clone())
        .chain(snapshot.tracks.iter().map(|t| t.0.clone()))
        .chain(snapshot.albums.iter().map(|a| a.0.clone()))
        .chain(snapshot.artists.iter().map(|a| a.0.clone()))
        .chain(snapshot.alias_ids.iter().cloned())
        .collect();
    let mut aliases = Vec::new();

    for file in &mut files {
        if file.row.id.is_empty() {
            file.row.id = fresh_id(IdKind::File, &mut taken);
        }
    }
    let infos: Vec<TrackInfo> = files
        .iter()
        .map(|f| TrackInfo::new(&f.tags, Path::new(&f.row.path)))
        .collect();

    // 曲
    let (track_keys, file_track) = group_by(infos.iter().map(|i| i.match_key.as_str()));
    let mut track_files: Vec<Vec<usize>> = vec![Vec::new(); track_keys.len()];
    for (file, &track) in file_track.iter().enumerate() {
        track_files[track].push(file);
    }
    let track_groups: Vec<Group> = track_keys
        .iter()
        .zip(&track_files)
        .map(|(key, members)| Group {
            key: key.clone(),
            previous: members
                .iter()
                .map(|&f| files[f].row.track_id.clone())
                .filter(|id| !id.is_empty())
                .collect(),
        })
        .collect();
    let existing_tracks: Vec<(String, String)> = snapshot
        .tracks
        .iter()
        .map(|(id, key, ..)| (id.clone(), key.clone()))
        .collect();
    let track_ids = assign_ids(
        IdKind::Track,
        &track_groups,
        &existing_tracks,
        &mut taken,
        &mut aliases,
    );
    let primaries: Vec<usize> = track_files
        .iter()
        .map(|members| primary_file(&files, members))
        .collect();

    // アルバム。曲の配信ファイルのタグで決める
    let (album_keys, track_album) =
        group_by(primaries.iter().map(|&f| infos[f].album.match_key.as_str()));
    let previous_album: HashMap<&str, &str> = snapshot
        .tracks
        .iter()
        .map(|(id, _, album, _)| (id.as_str(), album.as_str()))
        .collect();
    let mut album_tracks: Vec<Vec<usize>> = vec![Vec::new(); album_keys.len()];
    for (track, &album) in track_album.iter().enumerate() {
        album_tracks[album].push(track);
    }
    let album_groups: Vec<Group> = album_keys
        .iter()
        .zip(&album_tracks)
        .map(|(key, members)| Group {
            key: key.clone(),
            previous: members
                .iter()
                .filter_map(|&t| previous_album.get(track_ids[t].as_str()))
                .map(|id| (*id).to_owned())
                .collect(),
        })
        .collect();
    let existing_albums: Vec<(String, String)> = snapshot
        .albums
        .iter()
        .map(|(id, key, _)| (id.clone(), key.clone()))
        .collect();
    let album_ids = assign_ids(
        IdKind::Album,
        &album_groups,
        &existing_albums,
        &mut taken,
        &mut aliases,
    );

    // アーティスト。曲とアルバムの位置ごとに、前回そこにいたアーティストを引き継ぎの候補にする
    let track_credits: Vec<&[Credit]> = primaries.iter().map(|&f| &infos[f].artists[..]).collect();
    let album_credits: Vec<&[Credit]> = album_tracks
        .iter()
        .map(|tracks| &infos[primaries[tracks[0]]].album.artists[..])
        .collect();
    let previous_track_artist: HashMap<(&str, i64), &str> = snapshot
        .track_artists
        .iter()
        .map(|(t, p, a)| ((t.as_str(), *p), a.as_str()))
        .collect();
    let previous_album_artist: HashMap<(&str, i64), &str> = snapshot
        .album_artists
        .iter()
        .map(|(t, p, a)| ((t.as_str(), *p), a.as_str()))
        .collect();
    let mut slots: Vec<(&Credit, Option<&str>)> = Vec::new();
    for (credits, owner) in track_credits.iter().zip(&track_ids) {
        for (position, credit) in credits.iter().enumerate() {
            let previous = previous_track_artist.get(&(owner.as_str(), position as i64));
            slots.push((credit, previous.copied()));
        }
    }
    for (credits, owner) in album_credits.iter().zip(&album_ids) {
        for (position, credit) in credits.iter().enumerate() {
            let previous = previous_album_artist.get(&(owner.as_str(), position as i64));
            slots.push((credit, previous.copied()));
        }
    }
    let (artist_keys, slot_artist) = group_by(slots.iter().map(|(c, _)| c.match_key.as_str()));
    let mut artist_groups: Vec<Group> = artist_keys
        .iter()
        .map(|key| Group {
            key: key.clone(),
            previous: Vec::new(),
        })
        .collect();
    for ((_, previous), &artist) in slots.iter().zip(&slot_artist) {
        if let Some(previous) = previous {
            artist_groups[artist].previous.push((*previous).to_owned());
        }
    }
    let artist_ids = assign_ids(
        IdKind::Artist,
        &artist_groups,
        &snapshot.artists,
        &mut taken,
        &mut aliases,
    );
    let artist_by_key: HashMap<&str, &str> = artist_keys
        .iter()
        .zip(&artist_ids)
        .map(|(k, id)| (k.as_str(), id.as_str()))
        .collect();

    let artists = artist_rows(&infos, &artist_keys, &artist_ids);
    let created_at: HashMap<&str, i64> = snapshot
        .albums
        .iter()
        .map(|(id, _, at)| (id.as_str(), *at))
        .collect();
    let albums: Vec<AlbumRow> = album_keys
        .iter()
        .zip(&album_ids)
        .zip(&album_tracks)
        .map(|((key, id), tracks)| {
            let members: Vec<usize> = tracks
                .iter()
                .flat_map(|&t| track_files[t].iter().copied())
                .collect();
            album_row(
                id,
                key,
                &members,
                &files,
                &infos,
                created_at.get(id.as_str()).copied().unwrap_or(now),
            )
        })
        .collect();

    let track_created_at: HashMap<&str, i64> = snapshot
        .tracks
        .iter()
        .map(|(id, _, _, at)| (id.as_str(), *at))
        .collect();
    let mut tracks = Vec::new();
    let mut track_artists = Vec::new();
    let mut track_genres = Vec::new();
    for (t, id) in track_ids.iter().enumerate() {
        let primary = primaries[t];
        let info = &infos[primary];
        let reading = info.reading.as_ref();
        tracks.push(TrackRow {
            id: id.clone(),
            match_key: track_keys[t].clone(),
            album_id: album_ids[track_album[t]].clone(),
            title: info.title.clone(),
            display_artist: info.display_artist.clone(),
            sort_name: reading.map(|r| r.kana.clone()),
            sort_name_source: reading.map(|r| r.source.as_str().to_owned()),
            sort_key: sort_key(&info.title, reading.map(|r| r.kana.as_str())),
            disc_number: info.disc_number.map(i64::from),
            track_number: info.track_number.map(i64::from),
            year: info.year.map(i64::from),
            primary_file_id: files[primary].row.id.clone(),
            // 0 は、created_at を持つ前のマイグレーションで入った行
            created_at: track_created_at
                .get(id.as_str())
                .copied()
                .filter(|at| *at > 0)
                .unwrap_or(now),
        });
        track_artists.extend(credit_rows(id, &info.artists, &artist_by_key));
        track_genres.extend(info.genres.iter().enumerate().map(|(p, genre)| GenreRow {
            track_id: id.clone(),
            position: p as i64,
            genre: genre.clone(),
        }));
    }
    let album_artists = album_ids
        .iter()
        .zip(&album_credits)
        .flat_map(|(id, credits)| credit_rows(id, credits, &artist_by_key))
        .collect();

    let files = files
        .into_iter()
        .zip(&file_track)
        .map(|(file, &t)| FileRow {
            track_id: track_ids[t].clone(),
            tags: serde_json::to_string(&file.tags).expect("RawTags は JSON にできる"),
            ..file.row
        })
        .collect();

    Library {
        music_folder_path: music_dir.to_owned(),
        artists,
        albums,
        tracks,
        files,
        track_artists,
        album_artists,
        track_genres,
        aliases,
    }
}

/// 配信に使うファイル。可逆圧縮を優先し、同じ種類ならビットレートが高いほう（docs/schema.md）。
fn primary_file(files: &[Scanned], members: &[usize]) -> usize {
    let rank = |&f: &usize| (files[f].row.lossless, files[f].row.bit_rate.unwrap_or(0));
    let mut best = members[0];
    for &f in &members[1..] {
        // 同じ順位なら、パスの順で先のファイルを残す
        if rank(&f) > rank(&best) {
            best = f;
        }
    }
    best
}

fn credit_rows(
    owner: &str,
    credits: &[Credit],
    artist_by_key: &HashMap<&str, &str>,
) -> Vec<CreditRow> {
    credits
        .iter()
        .enumerate()
        .map(|(p, c)| CreditRow {
            owner_id: owner.to_owned(),
            position: p as i64,
            artist_id: artist_by_key[c.match_key.as_str()].to_owned(),
            credited_name: c.name.clone(),
            credited_sort: c.sort.clone(),
        })
        .collect()
}

/// アルバムの表記は、属するファイルの多数決で決める。年は最も新しい年。
fn album_row(
    id: &str,
    key: &str,
    members: &[usize],
    files: &[Scanned],
    infos: &[TrackInfo],
    created_at: i64,
) -> AlbumRow {
    let albums = || members.iter().map(|&f| &infos[f].album);
    let name = majority(albums().map(|a| a.name.as_str()))
        .unwrap_or_default()
        .to_owned();
    let display_artist = majority(albums().map(|a| a.display_artist.as_str()))
        .unwrap_or_default()
        .to_owned();
    let sort = majority(
        members
            .iter()
            .filter_map(|&f| files[f].tags.album_sort.as_deref()),
    );
    let reading = reading(&name, sort);
    AlbumRow {
        id: id.to_owned(),
        match_key: key.to_owned(),
        display_artist,
        sort_name: reading.as_ref().map(|r| r.kana.clone()),
        sort_name_source: reading.as_ref().map(|r| r.source.as_str().to_owned()),
        sort_key: sort_key(&name, reading.as_ref().map(|r| r.kana.as_str())),
        year: members
            .iter()
            .filter_map(|&f| infos[f].year)
            .max()
            .map(i64::from),
        created_at,
        compilation: members.iter().any(|&f| infos[f].album.compilation),
        name,
    }
}

/// アーティストの表記と読みは、そのアーティストが載っているファイルの多数決で決める。
/// 一つのファイルは、曲とアルバムの両方に載っていても一票とする。
fn artist_rows(infos: &[TrackInfo], keys: &[String], ids: &[String]) -> Vec<ArtistRow> {
    let index: HashMap<&str, usize> = keys
        .iter()
        .enumerate()
        .map(|(i, k)| (k.as_str(), i))
        .collect();
    let mut names: Vec<Vec<&str>> = vec![Vec::new(); keys.len()];
    let mut sorts: Vec<Vec<&str>> = vec![Vec::new(); keys.len()];
    for info in infos {
        let mut seen = HashSet::new();
        for credit in info.artists.iter().chain(&info.album.artists) {
            let Some(&i) = index.get(credit.match_key.as_str()) else {
                continue;
            };
            if !seen.insert(i) {
                continue;
            }
            names[i].push(&credit.name);
            if let Some(sort) = &credit.sort {
                sorts[i].push(sort);
            }
        }
    }
    keys.iter()
        .zip(ids)
        .enumerate()
        .map(|(i, (key, id))| {
            let name = majority(names[i].iter().copied())
                .unwrap_or_default()
                .to_owned();
            let reading = reading(&name, majority(sorts[i].iter().copied()));
            ArtistRow {
                id: id.clone(),
                match_key: key.clone(),
                sort_name: reading.as_ref().map(|r| r.kana.clone()),
                sort_name_source: reading.as_ref().map(|r| r.source.as_str().to_owned()),
                sort_key: sort_key(&name, reading.as_ref().map(|r| r.kana.as_str())),
                name,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn group(key: &str, previous: &[&str]) -> Group {
        Group {
            key: key.to_owned(),
            previous: previous.iter().map(|p| (*p).to_owned()).collect(),
        }
    }

    fn existing(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(id, key)| ((*id).to_owned(), (*key).to_owned()))
            .collect()
    }

    fn run(
        groups: &[Group],
        existing: &[(String, String)],
    ) -> (Vec<String>, Vec<(String, String)>) {
        let mut taken = existing.iter().map(|(id, _)| id.clone()).collect();
        let mut aliases = Vec::new();
        let ids = assign_ids(IdKind::Track, groups, existing, &mut taken, &mut aliases);
        (ids, aliases)
    }

    #[test]
    fn same_key_keeps_id() {
        let (ids, aliases) = run(&[group("a", &[])], &existing(&[("tr-1", "a")]));
        assert_eq!(ids, ["tr-1"]);
        assert!(aliases.is_empty());
    }

    #[test]
    fn edited_key_inherits_previous_id() {
        let (ids, aliases) = run(&[group("b", &["tr-1"])], &existing(&[("tr-1", "a")]));
        assert_eq!(ids, ["tr-1"]);
        assert!(aliases.is_empty());
    }

    #[test]
    fn previous_id_still_in_use_is_not_inherited() {
        // 二つのファイルの片方だけタグを直した
        let (ids, _) = run(
            &[group("a", &["tr-1"]), group("b", &["tr-1"])],
            &existing(&[("tr-1", "a")]),
        );
        assert_eq!(ids[0], "tr-1");
        assert_ne!(ids[1], "tr-1");
        assert!(ids[1].starts_with("tr-"));
    }

    #[test]
    fn merged_id_becomes_alias() {
        // タグを直したら別の曲と鍵が一致した
        let (ids, aliases) = run(
            &[group("a", &["tr-1", "tr-2"])],
            &existing(&[("tr-1", "a"), ("tr-2", "b")]),
        );
        assert_eq!(ids, ["tr-1"]);
        assert_eq!(aliases, [("tr-2".to_owned(), "tr-1".to_owned())]);
    }

    #[test]
    fn majority_prefers_first_on_tie() {
        assert_eq!(majority(["b", "a", "a", "b"].into_iter()), Some("b"));
        assert_eq!(majority(["b", "a", "a"].into_iter()), Some("a"));
        assert_eq!(majority(std::iter::empty::<&str>()), None);
    }
}

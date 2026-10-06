/** アーティストの詳細の表示（docs/web.md の「一覧」）。 */
import type { Album, Artist, Song } from '~/utils/subsonic'

/** どの一覧から開いたか。`albums` はアルバムアーティスト、`tracks` は曲のアーティスト、`composer` は作曲家の一覧。 */
export type ArtistView = 'albums' | 'tracks' | 'composer'

export function artistView(value: unknown): ArtistView {
  return value === 'tracks' || value === 'composer' ? value : 'albums'
}

/** 検索の結果から開く表示。役割に合わない表示で開くと曲が一つも出ないので、`roles` で選ぶ（docs/web.md の「検索」）。 */
export function viewForRoles(artist: Pick<Artist, 'roles'>): ArtistView {
  const roles = artist.roles
  if (!roles || roles.includes('artist')) {
    return 'tracks'
  }
  return roles.includes('albumartist') ? 'albums' : 'composer'
}

const composerRoles = ['composer', 'lyricist', 'arranger']

/**
 * 詳細で切り替えられる表示。曲やアルバムと、作曲などの両方で関わった人だけ二つ返し、ほかは空にする（docs/web.md の「一覧」）。
 * 曲やアルバムの側は、開いている表示がそちらならそれを、作曲家の表示なら役割から選ぶ。
 */
export function switchableViews(artist: Pick<Artist, 'roles'>, view: ArtistView): ArtistView[] {
  const roles = artist.roles ?? []
  const performs = roles.includes('artist') || roles.includes('albumartist')
  const composes = roles.some(role => composerRoles.includes(role))
  if (!performs || !composes) {
    return []
  }
  return [view === 'composer' ? viewForRoles(artist) : view, 'composer']
}

/** 表示ごとに `getArtist` の `songs` に渡す役割。 */
export const songRoles: Record<ArtistView, string | undefined> = {
  albums: undefined,
  tracks: 'artist',
  composer: 'composer,lyricist,arranger',
}

/** `artistId` がアルバムアーティストになっているアルバム。 */
export function albumsAsAlbumArtist(albums: Album[], artistId: string): Album[] {
  return albums.filter(album => album.artists?.some(artist => artist.id === artistId))
}

export interface SongGroup {
  albumId: string
  album: string
  coverArt?: string
  songs: Song[]
}

/** アルバムごとにまとまって並んだ曲を、アルバムの見出しごとに分ける。 */
export function groupByAlbum(songs: Song[]): SongGroup[] {
  const groups: SongGroup[] = []
  for (const song of songs) {
    const albumId = song.albumId ?? ''
    const last = groups.at(-1)
    if (last && last.albumId === albumId) {
      last.songs.push(song)
    }
    else {
      groups.push({ albumId, album: song.album ?? '', coverArt: song.coverArt, songs: [song] })
    }
  }
  return groups
}

/** 並び順。`name` はアルバム名、作曲家の表示では曲名の順。 */
export type ArtistSort = 'newest' | 'oldest' | 'name'

/** 表示ごとの並び順の選択肢。先頭が既定。 */
export const sortOptions: Record<ArtistView, { value: ArtistSort, label: string }[]> = {
  albums: [
    { value: 'newest', label: '新しい順' },
    { value: 'oldest', label: '古い順' },
    { value: 'name', label: 'アルバム名順' },
  ],
  tracks: [
    { value: 'newest', label: '新しい順' },
    { value: 'oldest', label: '古い順' },
    { value: 'name', label: 'アルバム名順' },
  ],
  composer: [
    { value: 'newest', label: '新しい順' },
    { value: 'oldest', label: '古い順' },
    { value: 'name', label: '曲名順' },
  ],
}

export function artistSort(value: unknown): ArtistSort {
  return value === 'oldest' || value === 'name' ? value : 'newest'
}

const collator = new Intl.Collator('ja')

/** 年で比べる。年のないものは、どちらの向きでも最後に回す。 */
function byYear(a: number | undefined, b: number | undefined, sort: ArtistSort): number {
  if (a === b) {
    return 0
  }
  if (a === undefined) {
    return 1
  }
  if (b === undefined) {
    return -1
  }
  return sort === 'newest' ? b - a : a - b
}

interface Sortable {
  year?: number
  /** 名前順に使う名前（読みがあれば読み） */
  key: string
}

function compare(a: Sortable, b: Sortable, sort: ArtistSort): number {
  if (sort === 'name') {
    return collator.compare(a.key, b.key)
  }
  return byYear(a.year, b.year, sort) || collator.compare(a.key, b.key)
}

export function sortAlbums(albums: Album[], sort: ArtistSort): Album[] {
  const key = (album: Album) => ({ year: album.year, key: album.sortName ?? album.name })
  return [...albums].sort((a, b) => compare(key(a), key(b), sort))
}

/** 見出しの順を、`albums` にある同じアルバムの年と読みで並べ替える。見出しの中はトラック番号の順のまま。 */
export function sortGroups(groups: SongGroup[], sort: ArtistSort, albums: Album[]): SongGroup[] {
  const byId = new Map(albums.map(album => [album.id, album]))
  const key = (group: SongGroup) => {
    const album = byId.get(group.albumId)
    return { year: album?.year ?? group.songs[0]?.year, key: album?.sortName ?? group.album }
  }
  return [...groups].sort((a, b) => compare(key(a), key(b), sort))
}

/** 年の順では、同じ年の曲はサーバーの順（アルバムごと、トラック番号の順）のまま並べる。 */
export function sortSongs(songs: Song[], sort: ArtistSort): Song[] {
  if (sort === 'name') {
    return [...songs].sort((a, b) => collator.compare(a.sortName ?? a.title, b.sortName ?? b.title))
  }
  return [...songs].sort((a, b) => byYear(a.year, b.year, sort))
}

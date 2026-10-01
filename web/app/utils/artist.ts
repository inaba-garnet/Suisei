/** アーティストの詳細の表示（docs/web.md の「一覧」）。 */
import type { Album, Song } from '~/utils/subsonic'

/** どの一覧から開いたか。`albums` はアルバムアーティスト、`tracks` は曲のアーティスト、`composer` は作曲家の一覧。 */
export type ArtistView = 'albums' | 'tracks' | 'composer'

export function artistView(value: unknown): ArtistView {
  return value === 'tracks' || value === 'composer' ? value : 'albums'
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

import type { SubsonicParams } from '~/composables/useSubsonic'
import type { Song } from '~/utils/subsonic'

/** 編集中の曲。同じ曲を何度でも入れられるので、元の位置を鍵にする。 */
export interface PlaylistRow {
  key: number
  song: Song
}

/**
 * 編集の結果を `updatePlaylist` の引数にする（docs/web.md の「プレイリスト」）。変わっていなければ `undefined`。
 * 曲の並びが変わったら、元の曲をすべて外して新しい並びで入れ直す。サーバーは外すのを先に、足すのを後に行う。
 */
export function playlistUpdate(id: string, name: string, original: { name: string, songs: Song[] }, rows: PlaylistRow[]): SubsonicParams | undefined {
  const params: SubsonicParams = { playlistId: id }
  const trimmed = name.trim()
  if (trimmed && trimmed !== original.name) {
    params.name = trimmed
  }
  const reordered = rows.length !== original.songs.length || rows.some((row, i) => row.key !== i)
  if (reordered) {
    params.songIndexToRemove = original.songs.map((_, i) => i)
    params.songIdToAdd = rows.map(row => row.song.id)
  }
  return Object.keys(params).length > 1 ? params : undefined
}

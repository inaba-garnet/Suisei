/** Subsonic API の応答の型と、呼び出しの補助（docs/web.md の「一覧」）。 */

export interface ArtistRef {
  id: string
  name: string
}

export interface Artist {
  id: string
  name: string
  albumCount: number
}

/** `getArtists` の、読みの行ごとの見出しとアーティスト。 */
export interface ArtistIndex {
  name: string
  artist?: Artist[]
}

export interface ArtistWithAlbums extends Artist {
  album?: Album[]
}

export interface Album {
  id: string
  name: string
  artist?: string
  artists?: ArtistRef[]
  coverArt?: string
  year?: number
  genres?: { name: string }[]
  songCount: number
  duration: number
}

export interface Song {
  id: string
  title: string
  artist?: string
  track?: number
  discNumber?: number
  duration: number
}

export interface AlbumWithSongs extends Album {
  song?: Song[]
}

/** 呼び出しで名乗るクライアント名と、対応する API の版。 */
export const CLIENT = 'suisei-web'
export const API_VERSION = '1.16.1'

/** Subsonic のエラー。`code` は Subsonic のエラーコード（40 は認証の失敗）。 */
export class SubsonicError extends Error {
  constructor(readonly code: number, message: string) {
    super(message)
  }
}

/** 応答から `subsonic-response` を取り出す。失敗の応答なら SubsonicError を投げる。 */
export function unwrap<T>(body: unknown): T {
  const res = (body as { 'subsonic-response'?: Record<string, unknown> } | null)?.['subsonic-response']
  if (!res) {
    throw new SubsonicError(0, 'サーバーの応答を読めません')
  }
  if (res.status !== 'ok') {
    const error = res.error as { code?: number, message?: string } | undefined
    throw new SubsonicError(error?.code ?? 0, error?.message ?? 'サーバーがエラーを返しました')
  }
  return res as T
}

/** ジャケットの URL。認証は Cookie のセッションに任せる。 */
export function coverArtUrl(id: string, size: number): string {
  return `/rest/getCoverArt?${new URLSearchParams({ id, size: String(size), c: CLIENT, v: API_VERSION })}`
}

/** 秒を `3:05` や `1:02:03` の形にする。 */
export function formatDuration(seconds: number): string {
  const total = Math.max(0, Math.round(seconds))
  const hours = Math.floor(total / 3600)
  const minutes = Math.floor((total % 3600) / 60)
  const rest = String(total % 60).padStart(2, '0')
  return hours > 0 ? `${hours}:${String(minutes).padStart(2, '0')}:${rest}` : `${minutes}:${rest}`
}

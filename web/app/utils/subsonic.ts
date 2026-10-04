/** Subsonic API の応答の型と、呼び出しの補助（docs/web.md の「一覧」）。 */

export interface ArtistRef {
  id: string
  name: string
}

export interface Artist {
  id: string
  name: string
  albumCount: number
  /** お気に入りにした日時。お気に入りでなければない */
  starred?: string
}

/** `getArtists` の、読みの行ごとの見出しとアーティスト。 */
export interface ArtistIndex {
  name: string
  artist?: Artist[]
}

export interface ArtistWithAlbums extends Artist {
  album?: Album[]
  /** 独自の引数 `songs` を渡したときだけ返る、その役割で関わった曲。 */
  song?: Song[]
}

export interface Album {
  id: string
  name: string
  sortName?: string
  /** お気に入りにした日時。お気に入りでなければない */
  starred?: string
  artist?: string
  artists?: ArtistRef[]
  coverArt?: string
  year?: number
  genres?: { name: string }[]
  songCount: number
  duration: number
}

/** `getGenres` のジャンル。 */
export interface Genre {
  value: string
  songCount: number
  albumCount: number
}

export interface Song {
  id: string
  title: string
  sortName?: string
  /** お気に入りにした日時。お気に入りでなければない */
  starred?: string
  year?: number
  artist?: string
  artists?: ArtistRef[]
  album?: string
  albumId?: string
  coverArt?: string
  track?: number
  discNumber?: number
  duration: number
  /** 元のファイルの拡張子と形式。ブラウザが鳴らせない形式を覚えるのに使う（docs/web.md の「再生」） */
  suffix?: string
  contentType?: string
}

export interface AlbumWithSongs extends Album {
  song?: Song[]
}

/** `getPlaylists` のプレイリスト。`coverArt` は画像のある最初の曲のアルバム。 */
export interface Playlist {
  id: string
  name: string
  comment?: string
  songCount: number
  duration: number
  coverArt?: string
  changed: string
}

export interface PlaylistWithSongs extends Playlist {
  entry?: Song[]
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

/** ブラウザが鳴らせない形式を変換させるときの形式。どのブラウザでも鳴り、サーバーは 320kbps で書き出す。 */
export const TRANSCODE_FORMAT = 'mp3'

/**
 * 曲の音声の URL。認証は Cookie のセッションに任せる（docs/web.md の「再生」）。
 * ふだんは元のファイルのまま受け取る。`transcode` なら MP3 に変換させ、`offset` 秒から頭出しさせる。
 */
export function streamUrl(id: string, transcode?: { offset: number }): string {
  const params = new URLSearchParams({ id, c: CLIENT, v: API_VERSION })
  if (transcode) {
    params.set('format', TRANSCODE_FORMAT)
    if (transcode.offset > 0) {
      params.set('timeOffset', transcode.offset.toFixed(3))
    }
  }
  return `/rest/stream?${params}`
}

/** 秒を `3:05` や `1:02:03` の形にする。 */
export function formatDuration(seconds: number): string {
  const total = Math.max(0, Math.round(seconds))
  const hours = Math.floor(total / 3600)
  const minutes = Math.floor((total % 3600) / 60)
  const rest = String(total % 60).padStart(2, '0')
  return hours > 0 ? `${hours}:${String(minutes).padStart(2, '0')}:${rest}` : `${minutes}:${rest}`
}

/** Spotify 連携の API（`/api/spotify`）の応答の型と、表示の補助（docs/web.md の「Spotify」）。 */

import { formatAgo } from '~/utils/time'

export interface SpotifyLastSync {
  /** 終わった日時（UNIX 時刻のミリ秒） */
  at: number
  error: string | null
  /** Spotify から読んだ曲の数。Spotify を呼ばずに付け直しただけなら null */
  fetched: number | null
  /** 新しく対応を付けた曲の数 */
  matched: number
}

export type SpotifyStatus
  = | { configured: false, redirectUri: string }
    | {
      configured: true
      redirectUri: string
      connected: boolean
      syncing: boolean
      lastSync: SpotifyLastSync | null
      total: number
      matched: number
    }

/** 対応表の一行。 */
export interface SpotifyTrack {
  id: string
  title: string
  artists: string[]
  album: string
  isrc: string | null
  durationMs: number
  addedAt: number
  trackId: string | null
  /** `isrc`、`match_key`、`fuzzy`、`manual`、`ignored`（手動で外した） */
  method: string | null
}

/** 貼り付けた URL を受け付けなかったときに、サーバーが返す理由。 */
export type SpotifyCallbackError = 'invalidUrl' | 'denied' | 'unknownState' | 'spotify'

/** 貼り付けた URL を受け付けなかった理由の文。 */
export function callbackErrorMessage(error: string | undefined): string {
  switch (error) {
    case 'invalidUrl':
      return 'URL を読めません。アドレスバーの URL をそのまま貼り付けてください'
    case 'denied':
      return 'Spotify で許可されませんでした'
    case 'unknownState':
      return '古い URL です。もう一度「Spotify に接続」からやり直してください'
    default:
      return 'Spotify につながりませんでした。しばらくしてからやり直してください'
  }
}

/** 最後の取り込みの結果の文。 */
export function lastSyncMessage(last: SpotifyLastSync, now: number): string {
  const when = formatAgo(last.at, now)
  if (last.error) {
    return `${when}、取り込みに失敗しました（${last.error}）`
  }
  return last.matched > 0
    ? `${when}、${last.matched} 曲をお気に入りにしました`
    : `${when}、新しく取り込んだ曲はありませんでした`
}

/** 未対応の曲の行から、ローカルの曲を探す検索語を作る。曲名だけにする。アーティストの表記は Spotify とローカルで違うことが多いため。 */
export function searchTermOf(track: Pick<SpotifyTrack, 'title'>): string {
  return track.title.trim()
}

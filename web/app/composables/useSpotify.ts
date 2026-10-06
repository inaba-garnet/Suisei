import type { SpotifyStatus, SpotifyTrack } from '~/utils/spotify'

/**
 * Spotify 連携の API（docs/spotify.md の「API」）を呼ぶ関数をまとめて返す。
 * 401 ならセッションが切れたとみなし、ログインの画面に移す（docs/web.md の「一覧」と同じ扱い）。
 */
export function useSpotify() {
  const user = useUser()
  const route = useRoute()

  async function call<T>(path: string, options: Parameters<typeof $fetch>[1] = {}): Promise<T> {
    try {
      return await $fetch<T>(`/api/spotify${path}`, options as never)
    }
    catch (err) {
      if (statusOf(err) === 401) {
        user.value = null
        await navigateTo({ path: '/login', query: { redirect: route.fullPath } })
      }
      throw err
    }
  }

  // 書き込む要求は JSON で送る。$fetch は body がないと Content-Type を付けないので、空の JSON を送る
  return {
    status: () => call<SpotifyStatus>(''),
    authorize: () => call<{ url: string }>('/authorize', { method: 'POST', body: {} }),
    callback: (url: string) => call<undefined>('/callback', { method: 'POST', body: { url } }),
    sync: () => call<undefined>('/sync', { method: 'POST', body: {} }),
    disconnect: () => call<undefined>('', { method: 'DELETE', body: {} }),
    tracks: (matched?: boolean) =>
      call<{ tracks: SpotifyTrack[] }>('/tracks', { query: matched === undefined ? {} : { matched } }),
    link: (spotifyId: string, trackId: string | null) =>
      call<undefined>(`/tracks/${encodeURIComponent(spotifyId)}`, { method: 'PUT', body: { trackId } }),
  }
}

function statusOf(err: unknown): number | undefined {
  return (err as { response?: { status?: number } }).response?.status
}

/** `/api/spotify` の失敗した応答の JSON の `error`。 */
export function spotifyErrorOf(err: unknown): string | undefined {
  return (err as { data?: { error?: string } }).data?.error
}

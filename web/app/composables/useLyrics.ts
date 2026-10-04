import type { StructuredLyrics } from '~/utils/subsonic'

/** 曲ごとに取った歌詞。曲を戻したときに取り直さないよう、ページを読み直すまで覚える。 */
const cache = new Map<string, Promise<StructuredLyrics | null>>()

/**
 * 再生中の曲の歌詞（docs/web.md の「再生」）。曲が変わるたびに `getLyricsBySongId` で取る。
 * 取っている間は `undefined`、ない曲は `null`。サーバーは時刻付きを先に並べるので、行のある最初の歌詞を出す。
 */
export function useLyrics() {
  const { current } = usePlayer()
  const subsonic = useSubsonic()
  const lyrics = shallowRef<StructuredLyrics | null>()

  function load(id: string): Promise<StructuredLyrics | null> {
    let request = cache.get(id)
    if (!request) {
      request = subsonic<{ lyricsList: { structuredLyrics?: StructuredLyrics[] } }>('getLyricsBySongId', { id })
        .then(res => res.lyricsList.structuredLyrics?.find(l => l.line.some(line => line.value.trim())) ?? null)
      // 取れなかったら覚えず、次にその曲を鳴らしたときに取り直す
      request.catch(() => cache.delete(id))
      cache.set(id, request)
    }
    return request
  }

  watch(() => current.value?.id, async (id) => {
    lyrics.value = undefined
    if (!id) {
      return
    }
    const result = await load(id).catch(() => null)
    if (current.value?.id === id) {
      lyrics.value = result
    }
  }, { immediate: true })

  return lyrics
}

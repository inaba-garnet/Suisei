import type { PlayerState } from './usePlayer'
import { useEventListener } from '@vueuse/core'

/** これだけ聴けば、曲の半分に届かなくても再生回数に数える（秒）。 */
const SCROBBLE_AFTER = 240

/**
 * 聴いた曲を `scrobble` で再生回数に数える（docs/web.md の「再生」）。
 * 曲の半分か 4 分の早い方まで聴いたら、頭から鳴らし始めるたびに一度だけ送る。シークで飛ばした分は聴いたうちに入れない。
 */
export function usePlayerScrobble(el: Ref<HTMLAudioElement | undefined>) {
  const state = useState<PlayerState>('player')
  const subsonic = useSubsonic()

  /** 頭から鳴らし始めた時刻（ミリ秒） */
  let startedAt = 0
  /** 聴いた秒数 */
  let listened = 0
  /** 前の timeupdate の再生位置。シークの直後は捨て、次の timeupdate から数え直す */
  let last: number | undefined
  let sent = false

  watch(() => state.value.started, () => {
    startedAt = Date.now()
    listened = 0
    last = undefined
    sent = false
  })

  useEventListener(el, 'seeking', () => {
    last = undefined
  })

  useEventListener(el, 'timeupdate', () => {
    const audio = el.value
    const song = state.value.queue[state.value.index]
    if (!audio || !song || sent) {
      return
    }
    const now = audio.currentTime
    if (last !== undefined && now > last) {
      listened += now - last
    }
    last = now
    // 実際に鳴らす長さを先に見る。タグの長さが実際より長いと、最後まで聴いても半分に届かないことがあるため。
    // 変換して鳴らしている曲は、`<audio>` の長さが頭出しした位置からの残りになるので、曲の長さを使う
    const duration = !state.value.transcoding && Number.isFinite(audio.duration) && audio.duration > 0 ? audio.duration : song.duration
    if (listened < Math.min(duration / 2, SCROBBLE_AFTER)) {
      return
    }
    sent = true
    // 失敗しても再送しない。再生を妨げないため
    subsonic('scrobble', { id: song.id, time: startedAt, submission: 'true' }).catch(() => {})
  })
}

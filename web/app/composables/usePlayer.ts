import type { Song } from '~/utils/subsonic'
import { useLocalStorage } from '@vueuse/core'
import { streamUrl } from '~/utils/subsonic'

/** リピートの種類。`all` はキュー全体、`one` は再生中の曲だけを繰り返す。 */
export type RepeatMode = 'off' | 'all' | 'one'

/** 再生の状態（docs/web.md の「再生」）。画面を移っても途切れさせないため、ページの外に持つ。 */
export interface PlayerState {
  queue: Song[]
  /** キューの中の再生中の曲。何も再生していなければ -1 */
  index: number
  playing: boolean
  /** 再生位置（秒） */
  position: number
  /** 再生中の曲を鳴らせなかったか */
  failed: boolean
  shuffle: boolean
  /** シャッフルする前のキュー。オフに戻すときに使う */
  original: Song[]
  repeat: RepeatMode
}

/** 前の曲を押したとき、この秒数より先まで進んでいれば曲の頭に戻す。 */
const RESTART_AFTER = 3

/** 選んだ音量（0〜1）を残す `localStorage` の名前。 */
const VOLUME_STORAGE_KEY = 'suisei-volume'

/** `songs` の `first` 番目を先頭に置き、残りを混ぜる。 */
function shuffled(songs: Song[], first: number): Song[] {
  const rest = songs.filter((_, i) => i !== first)
  for (let i = rest.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1))
    ;[rest[i], rest[j]] = [rest[j]!, rest[i]!]
  }
  const head = songs[first]
  return head ? [head, ...rest] : rest
}

/** layout に一つだけ置く `<audio>`（PlayerAudio）。SPA なので、モジュールに持っても利用者の間で混ざらない。 */
const audio = shallowRef<HTMLAudioElement>()

function initial(): PlayerState {
  return { queue: [], index: -1, playing: false, position: 0, failed: false, shuffle: false, original: [], repeat: 'off' }
}

export function usePlayer() {
  const state = useState<PlayerState>('player', initial)
  const current = computed<Song | undefined>(() => state.value.queue[state.value.index])

  /** キューの `index` 番目の曲を読み込んで鳴らす。 */
  function load(index: number) {
    const el = audio.value
    const song = state.value.queue[index]
    if (!el || !song) {
      return
    }
    state.value.index = index
    state.value.position = 0
    state.value.failed = false
    el.src = streamUrl(song.id)
    resume()
  }

  /** `songs` をキューにして、`index` 番目から鳴らす。シャッフル中なら、その曲を先頭に残して混ぜる。 */
  function start(songs: Song[], index = 0) {
    state.value.original = [...songs]
    if (state.value.shuffle) {
      state.value.queue = shuffled(songs, index)
      load(0)
    }
    else {
      state.value.queue = [...songs]
      load(index)
    }
  }

  function resume() {
    // 鳴らせなかったときは error か pause の event で状態を戻すので、ここでは失敗を捨てる
    audio.value?.play().catch(() => {})
  }

  function toggle() {
    const el = audio.value
    if (!el || !current.value) {
      return
    }
    if (el.paused) {
      resume()
    }
    else {
      el.pause()
    }
  }

  /** 次の曲に移る。キューの最後なら、キュー全体のリピートのときだけ先頭に戻る。 */
  function next() {
    if (state.value.index < state.value.queue.length - 1) {
      load(state.value.index + 1)
    }
    else if (state.value.repeat === 'all' && state.value.queue.length > 0) {
      load(0)
    }
  }

  function previous() {
    const el = audio.value
    if (!el || !current.value) {
      return
    }
    if (state.value.index === 0 || el.currentTime > RESTART_AFTER) {
      el.currentTime = 0
      return
    }
    load(state.value.index - 1)
  }

  /** 再生位置を `seconds` 秒に移す。 */
  function seek(seconds: number) {
    const el = audio.value
    if (!el || !current.value) {
      return
    }
    el.currentTime = seconds
    state.value.position = seconds
  }

  /** シャッフルを切り替える。再生中の曲はそのまま鳴らし続ける。 */
  function toggleShuffle() {
    const playing = current.value
    if (state.value.shuffle) {
      state.value.shuffle = false
      state.value.queue = [...state.value.original]
      state.value.index = playing ? Math.max(0, state.value.original.indexOf(playing)) : -1
      return
    }
    state.value.shuffle = true
    state.value.original = [...state.value.queue]
    if (playing) {
      state.value.queue = shuffled(state.value.queue, state.value.index)
      state.value.index = 0
    }
  }

  /** リピートを「なし」「キュー全体」「1 曲」の順に切り替える。 */
  function cycleRepeat() {
    const order: RepeatMode[] = ['off', 'all', 'one']
    state.value.repeat = order[(order.indexOf(state.value.repeat) + 1) % order.length]!
  }

  return {
    state: readonly(state),
    current,
    /** 前の曲は曲の頭に戻すこともあるので、何か再生していれば押せる */
    canPrevious: computed(() => !!current.value),
    canNext: computed(() => state.value.index < state.value.queue.length - 1 || (state.value.repeat === 'all' && !!current.value)),
    start,
    toggle,
    next,
    previous,
    seek,
    toggleShuffle,
    cycleRepeat,
  }
}

/** 音量（0〜1）。PC だけで変え、`localStorage` に残す（docs/web.md の「再生」）。 */
export function usePlayerVolume() {
  const volume = useLocalStorage(VOLUME_STORAGE_KEY, 1)
  watchEffect(() => {
    if (audio.value) {
      audio.value.volume = Math.min(1, Math.max(0, Number(volume.value) || 0))
    }
  })
  return volume
}

/** PlayerAudio が `<audio>` を渡し、その event で状態を更新する。 */
export function usePlayerAudio() {
  const state = useState<PlayerState>('player', initial)
  const { next } = usePlayer()

  function attach(el: HTMLAudioElement) {
    audio.value = el
  }

  /** layout が外れる（ログアウトなど）ときは鳴らすものがなくなるので、キューごと捨てる。 */
  function detach() {
    audio.value?.pause()
    audio.value = undefined
    state.value = initial()
  }

  const handlers = {
    play: () => {
      state.value.playing = true
    },
    pause: () => {
      state.value.playing = false
    },
    timeupdate: (e: Event) => {
      state.value.position = (e.target as HTMLAudioElement).currentTime
    },
    // 1 曲のリピートなら同じ曲を頭から鳴らす。キューの最後の曲が終わったら、リピートがなければその曲を出したまま止める
    ended: () => {
      if (state.value.repeat === 'one' && audio.value) {
        audio.value.currentTime = 0
        audio.value.play().catch(() => {})
        return
      }
      next()
    },
    error: () => {
      state.value.playing = false
      state.value.failed = true
    },
  }

  return { attach, detach, handlers }
}

import type { Song } from '~/utils/subsonic'
import { streamUrl } from '~/utils/subsonic'

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
}

/** 前の曲を押したとき、この秒数より先まで進んでいれば曲の頭に戻す。 */
const RESTART_AFTER = 3

/** layout に一つだけ置く `<audio>`（PlayerAudio）。SPA なので、モジュールに持っても利用者の間で混ざらない。 */
const audio = shallowRef<HTMLAudioElement>()

function initial(): PlayerState {
  return { queue: [], index: -1, playing: false, position: 0, failed: false }
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

  /** `songs` をキューにして、`index` 番目から鳴らす。 */
  function start(songs: Song[], index = 0) {
    state.value.queue = [...songs]
    load(index)
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

  function next() {
    if (state.value.index < state.value.queue.length - 1) {
      load(state.value.index + 1)
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

  return {
    state: readonly(state),
    current,
    /** 前の曲は曲の頭に戻すこともあるので、何か再生していれば押せる */
    canPrevious: computed(() => !!current.value),
    canNext: computed(() => state.value.index < state.value.queue.length - 1),
    start,
    toggle,
    next,
    previous,
  }
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
    // キューの最後の曲が終わったら、その曲を出したまま止める
    ended: next,
    error: () => {
      state.value.playing = false
      state.value.failed = true
    },
  }

  return { attach, detach, handlers }
}

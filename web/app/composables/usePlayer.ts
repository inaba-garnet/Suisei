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
  /** 曲を頭から鳴らし始めた回数。1 曲のリピートや曲の頭に戻したときも増やし、再生回数を数え直す合図にする */
  started: number
  /** 再生中の曲を、ブラウザが鳴らせないので変換して鳴らしているか */
  transcoding: boolean
  /** 変換して鳴らしているとき、頭出しした位置（秒）。再生位置は、これに `<audio>` で進んだ長さを足したもの */
  offset: number
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

/** ブラウザが鳴らせないと分かった形式。ページを読み直すまで覚え、同じ形式の曲は最初から変換する。 */
const unplayable = new Set<string>()
/** 鳴らせなかったので変換で読み直している曲の形式。変換で鳴ったら `unplayable` に足す */
let fallingBack: string | undefined

/**
 * 覚えない形式。`audio/mp4` は鳴らせない ALAC と鳴らせる AAC のどちらでもあり、ALAC で覚えると AAC まで変換してしまうため。
 * この形式の曲は、毎回まず元のファイルを試す。
 */
const AMBIGUOUS_TYPES = new Set(['audio/mp4'])

/** 形式を見分ける鍵。覚えない形式なら undefined。 */
function formatKey(song: Song) {
  if (AMBIGUOUS_TYPES.has(song.contentType ?? '')) {
    return undefined
  }
  return `${song.contentType ?? ''}|${song.suffix ?? ''}`
}

function initial(): PlayerState {
  return { queue: [], index: -1, playing: false, position: 0, failed: false, shuffle: false, original: [], repeat: 'off', started: 0, transcoding: false, offset: 0 }
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
    state.value.started++
    fallingBack = undefined
    const key = formatKey(song)
    openSong(song, key !== undefined && unplayable.has(key), 0, true)
  }

  /** 曲を `<audio>` に渡す。`transcode` なら MP3 に変換させ、`offset` 秒から頭出しさせる。 */
  function openSong(song: Song, transcode: boolean, offset: number, play: boolean) {
    const el = audio.value
    if (!el) {
      return
    }
    state.value.transcoding = transcode
    state.value.offset = transcode ? offset : 0
    el.src = streamUrl(song.id, transcode ? { offset } : undefined)
    if (play) {
      resume()
    }
  }

  /** 再生中の曲を頭から鳴らし直す。再生回数も数え直す。 */
  function restart() {
    const el = audio.value
    const song = current.value
    if (!el || !song) {
      return
    }
    state.value.started++
    state.value.position = 0
    if (state.value.transcoding) {
      openSong(song, true, 0, !el.paused || el.ended)
    }
    else {
      el.currentTime = 0
    }
  }

  /**
   * 鳴らせなかった曲を、変換して読み直す（docs/web.md の「再生」）。読み直したら true。
   * 変換しても鳴らせなかったときは false で、呼び出し側が鳴らせなかったことを出す。
   */
  function fallback() {
    const song = current.value
    if (!song || state.value.transcoding) {
      return false
    }
    fallingBack = formatKey(song)
    openSong(song, true, state.value.position, true)
    return true
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
    if (state.value.index === 0 || state.value.position > RESTART_AFTER) {
      restart()
      return
    }
    load(state.value.index - 1)
  }

  /** 再生位置を `seconds` 秒に移す。変換して鳴らしている曲は途中から読めないので、その位置から変換し直させる。 */
  function seek(seconds: number) {
    const el = audio.value
    const song = current.value
    if (!el || !song) {
      return
    }
    if (state.value.transcoding) {
      openSong(song, true, seconds, !el.paused)
    }
    else {
      el.currentTime = seconds
    }
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

  /** キューの `index` 番目の曲に移って鳴らす。 */
  function jump(index: number) {
    if (index !== state.value.index) {
      load(index)
    }
  }

  /**
   * キューの `from` 番目の曲を `to` 番目に移す。再生中の曲は鳴らし続け、その位置を合わせる。
   * シャッフル前の並び（`original`）は変えないので、シャッフルをオフにすると元の並びに戻る。
   */
  function move(from: number, to: number) {
    const queue = state.value.queue
    const playing = state.value.index
    if (from === to || !queue[from] || to < 0 || to >= queue.length) {
      return
    }
    const [song] = queue.splice(from, 1)
    queue.splice(to, 0, song!)
    if (from === playing) {
      state.value.index = to
    }
    else if (from < playing && to >= playing) {
      state.value.index--
    }
    else if (from > playing && to <= playing) {
      state.value.index++
    }
  }

  /** キューの `index` 番目の曲を消す。再生中の曲は消さない。シャッフル前の並びからも消す。 */
  function remove(index: number) {
    const song = state.value.queue[index]
    if (!song || index === state.value.index) {
      return
    }
    state.value.queue.splice(index, 1)
    if (index < state.value.index) {
      state.value.index--
    }
    const i = state.value.original.indexOf(song)
    if (i >= 0) {
      state.value.original.splice(i, 1)
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
    jump,
    move,
    remove,
    restart,
    fallback,
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
  const { next, restart, fallback } = usePlayer()

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
    // 変換して鳴らす曲を変換で鳴らせたら、その形式は鳴らせないと覚える
    playing: () => {
      if (fallingBack !== undefined && state.value.transcoding) {
        unplayable.add(fallingBack)
      }
      fallingBack = undefined
    },
    timeupdate: (e: Event) => {
      state.value.position = state.value.offset + (e.target as HTMLAudioElement).currentTime
    },
    // 1 曲のリピートなら同じ曲を頭から鳴らす。キューの最後の曲が終わったら、リピートがなければその曲を出したまま止める
    ended: () => {
      if (state.value.repeat === 'one' && audio.value) {
        restart()
        audio.value.play().catch(() => {})
        return
      }
      next()
    },
    // 元のファイルを鳴らせなければ変換で読み直し、それでも鳴らせなければ再生バーに出す
    error: () => {
      if (fallback()) {
        return
      }
      state.value.playing = false
      state.value.failed = true
    },
  }

  return { attach, detach, handlers }
}

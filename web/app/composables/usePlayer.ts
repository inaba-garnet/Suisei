import type { Song } from '~/utils/subsonic'
import { useLocalStorage } from '@vueuse/core'
import { streamUrl } from '~/utils/subsonic'

/** リピートの種類。`all` はキュー全体、`one` は再生中の曲だけを繰り返す。 */
export type RepeatMode = 'off' | 'all' | 'one'

/**
 * 再生中の曲を鳴らせなかった理由。`unplayable` は変換しても鳴らせなかった、
 * `blocked` はブラウザが `play()` を拒んだ（バックグラウンドで再生を始めさせないなど）。
 */
export type PlayFailure = 'unplayable' | 'blocked'

/** 再生バーなどに出す、鳴らせなかった理由。 */
export const PLAY_FAILURE_MESSAGES: Record<PlayFailure, string> = {
  unplayable: '再生できませんでした',
  blocked: 'ブラウザに再生を止められました',
}

/** 再生の状態（docs/web.md の「再生」）。画面を移っても途切れさせないため、ページの外に持つ。 */
export interface PlayerState {
  queue: Song[]
  /** キューの中の再生中の曲。何も再生していなければ -1 */
  index: number
  playing: boolean
  /** 再生位置（秒） */
  position: number
  /** 再生中の曲を鳴らせなかった理由。鳴らせていれば null */
  failure: PlayFailure | null
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

/**
 * 曲の残りがこの秒数を切ったら、次の曲を控えの `<audio>` で読み始める（#143）。
 * 曲の切れ目で読み込みを待つ無音を作らないため。Android の Chrome は、音が止まって数秒経ったタブの処理と通信を抑え、
 * 画面が消えている間に次の曲を読めずに止まる。
 */
const PRELOAD_BEFORE = 30

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

/**
 * layout に一つだけ置く PlayerAudio の、鳴らしている方の `<audio>`。SPA なので、モジュールに持っても利用者の間で混ざらない。
 * PlayerAudio は `<audio>` を二つ持ち、もう一方（`standby`）で次の曲を先に読む。曲が替わるときに入れ替える。
 */
const audio = shallowRef<HTMLAudioElement>()
const standby = shallowRef<HTMLAudioElement>()
/** 控えの `<audio>` に読んである曲。キューの曲のオブジェクトで、次に鳴らす曲と同じかを見る */
let preloaded: { song: Song, transcode: boolean } | undefined

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

/**
 * キューに足す曲の写し。シャッフル前の並びは曲のオブジェクトで探すので、
 * 同じ曲がキューに二度入っても取り違えないよう、足すたびに別のオブジェクトにする。
 */
function copiesOf(songs: Song[]): Song[] {
  return songs.map(song => ({ ...song }))
}

function initial(): PlayerState {
  return { queue: [], index: -1, playing: false, position: 0, failure: null, shuffle: false, original: [], repeat: 'off', started: 0, transcoding: false, offset: 0 }
}

export function usePlayer() {
  const state = useState<PlayerState>('player', initial)
  const current = computed<Song | undefined>(() => state.value.queue[state.value.index])

  /** 曲を変換して鳴らすか。鳴らせないと分かった形式なら、最初から変換する。 */
  function shouldTranscode(song: Song) {
    const key = formatKey(song)
    return key !== undefined && unplayable.has(key)
  }

  /** キューの `index` 番目の曲を読み込んで鳴らす。控えの `<audio>` に読んであれば、入れ替えてすぐ鳴らす。 */
  function load(index: number) {
    const el = audio.value
    const song = state.value.queue[index]
    if (!el || !song) {
      return
    }
    state.value.index = index
    state.value.position = 0
    state.value.failure = null
    state.value.started++
    fallingBack = undefined
    const ready = standby.value
    if (ready && preloaded?.song === song && !ready.error) {
      audio.value = ready
      standby.value = el
      state.value.transcoding = preloaded.transcode
      state.value.offset = 0
      preloaded = undefined
      resume()
      clear(el)
      return
    }
    clearStandby()
    openSong(song, shouldTranscode(song), 0, true)
  }

  /** 次に鳴らすキューの位置。次がなければ -1。 */
  function nextIndex() {
    if (state.value.index < state.value.queue.length - 1) {
      return state.value.index + 1
    }
    return state.value.repeat === 'all' && state.value.queue.length > 0 ? 0 : -1
  }

  /**
   * 次の曲を控えの `<audio>` で読み始める。読んである曲が次の曲と違えば（キューを並べ替えたときなど）読み直す。
   * 1 曲のリピートでは同じ `<audio>` を頭に戻すので読まない。
   */
  function preloadNext() {
    const el = standby.value
    const song = state.value.repeat === 'one' ? undefined : state.value.queue[nextIndex()]
    if (!el || !song || preloaded?.song === song) {
      return
    }
    const transcode = shouldTranscode(song)
    preloaded = { song, transcode }
    el.preload = 'auto'
    el.src = streamUrl(song.id, transcode ? { offset: 0 } : undefined)
    el.load()
  }

  /** `<audio>` の読み込みを止めて空にする。読みかけの曲の取得を続けさせないため。 */
  function clear(el: HTMLAudioElement) {
    el.pause()
    el.removeAttribute('src')
    el.load()
  }

  function clearStandby() {
    preloaded = undefined
    if (standby.value?.hasAttribute('src')) {
      clear(standby.value)
    }
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
    const el = audio.value
    if (!el) {
      return
    }
    const song = current.value
    el.play().catch((err: unknown) => {
      const name = err instanceof DOMException ? err.name : 'Error'
      // 止まった理由を実機で切り分けられるよう、拒否はすべて残す（#143）
      console.warn('[player] play() was rejected', {
        name,
        message: err instanceof Error ? err.message : String(err),
        visibility: document.visibilityState,
        src: el.currentSrc,
      })
      // AbortError は読み込みの途中で曲を替えたとき、NotSupportedError は鳴らせない形式で、error の event が扱う
      if (name === 'AbortError' || name === 'NotSupportedError' || current.value !== song) {
        return
      }
      state.value.playing = false
      state.value.failure = 'blocked'
    })
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
    const index = nextIndex()
    if (index >= 0) {
      load(index)
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

  /**
   * `songs` を再生中の曲の直後に入れる（docs/web.md の「メニュー」）。シャッフル前の並びでも再生中の曲の直後に入れ、
   * シャッフルをオフにしても続けて鳴るようにする。キューが空なら、その曲から鳴らし始める。
   */
  function playNext(songs: Song[]) {
    if (!songs.length) {
      return
    }
    const playing = current.value
    if (!playing) {
      start(songs)
      return
    }
    const copies = copiesOf(songs)
    state.value.queue.splice(state.value.index + 1, 0, ...copies)
    const i = state.value.original.indexOf(playing)
    state.value.original.splice(i >= 0 ? i + 1 : state.value.original.length, 0, ...copies)
  }

  /** `songs` をキューの最後に入れる。キューが空なら、その曲から鳴らし始める。 */
  function addToQueue(songs: Song[]) {
    if (!songs.length) {
      return
    }
    if (!current.value) {
      start(songs)
      return
    }
    const copies = copiesOf(songs)
    state.value.queue.push(...copies)
    state.value.original.push(...copies)
  }

  /** リピートを「なし」「キュー全体」「1 曲」の順に切り替える。 */
  function cycleRepeat() {
    const order: RepeatMode[] = ['off', 'all', 'one']
    state.value.repeat = order[(order.indexOf(state.value.repeat) + 1) % order.length]!
  }

  return {
    state: readonly(state),
    current,
    /** 再生中の曲を鳴らせなかった理由の文。鳴らせていれば undefined */
    failureMessage: computed(() => state.value.failure ? PLAY_FAILURE_MESSAGES[state.value.failure] : undefined),
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
    playNext,
    addToQueue,
    restart,
    fallback,
    resume,
    preloadNext,
  }
}

/** 音量（0〜1）。PC だけで変え、`localStorage` に残す（docs/web.md の「再生」）。 */
export function usePlayerVolume() {
  const volume = useLocalStorage(VOLUME_STORAGE_KEY, 1)
  // 控えの `<audio>` にも当て、入れ替えたときに音量が変わらないようにする
  watchEffect(() => {
    for (const el of [audio.value, standby.value]) {
      if (el) {
        el.volume = Math.min(1, Math.max(0, Number(volume.value) || 0))
      }
    }
  })
  return volume
}

/** 鳴らしている方の `<audio>`。曲が替わると控えと入れ替わるので、event はこれに付け直す。 */
export function useActiveAudio(): Readonly<Ref<HTMLAudioElement | undefined>> {
  return readonly(audio) as Readonly<Ref<HTMLAudioElement | undefined>>
}

/** PlayerAudio が `<audio>` を二つ渡し、鳴らしている方の event で状態を更新する。 */
export function usePlayerAudio() {
  const state = useState<PlayerState>('player', initial)
  const { current, next, restart, fallback, resume, preloadNext } = usePlayer()

  function attach(first: HTMLAudioElement, second: HTMLAudioElement) {
    audio.value = first
    standby.value = second
  }

  /** layout が外れる（ログアウトなど）ときは鳴らすものがなくなるので、キューごと捨てる。 */
  function detach() {
    audio.value?.pause()
    standby.value?.pause()
    audio.value = undefined
    standby.value = undefined
    preloaded = undefined
    state.value = initial()
  }

  /** 鳴らしている方の `<audio>` の event だけを扱う。控えの読み込みや、入れ替えた後の前の曲の event は捨てる。 */
  function active(handler: (el: HTMLAudioElement) => void) {
    return (e: Event) => {
      if (e.target === audio.value) {
        handler(e.target as HTMLAudioElement)
      }
    }
  }

  const handlers = {
    play: active(() => {
      state.value.playing = true
    }),
    pause: active(() => {
      state.value.playing = false
    }),
    // 変換して鳴らす曲を変換で鳴らせたら、その形式は鳴らせないと覚える。前に拒まれていても、鳴り出したら消す
    playing: active(() => {
      state.value.failure = null
      if (fallingBack !== undefined && state.value.transcoding) {
        unplayable.add(fallingBack)
      }
      fallingBack = undefined
    }),
    // 残りが短くなったら次の曲を読み始める。変換して鳴らしている曲は、`<audio>` の長さが頭出しした位置からの残りなので、曲の長さで見る
    timeupdate: active((el) => {
      state.value.position = state.value.offset + el.currentTime
      const duration = !state.value.transcoding && Number.isFinite(el.duration) ? el.duration : current.value?.duration
      if (duration !== undefined && duration - state.value.position <= PRELOAD_BEFORE) {
        preloadNext()
      }
    }),
    // 1 曲のリピートなら同じ曲を頭から鳴らす。キューの最後の曲が終わったら、リピートがなければその曲を出したまま止める
    ended: active(() => {
      if (state.value.repeat === 'one') {
        restart()
        resume()
        return
      }
      next()
    }),
    // 元のファイルを鳴らせなければ変換で読み直し、それでも鳴らせなければ再生バーに出す。控えの読み込みの失敗は、鳴らすときに読み直す
    error: active(() => {
      if (fallback()) {
        return
      }
      state.value.playing = false
      state.value.failure = 'unplayable'
    }),
  }

  return { attach, detach, handlers }
}

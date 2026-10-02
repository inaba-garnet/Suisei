import { useEventListener } from '@vueuse/core'
import { coverArtUrl } from '~/utils/subsonic'

/** ロック画面などに出すジャケットの大きさ。 */
const ARTWORK_SIZES = [256, 512]

/**
 * Media Session API で、再生中の曲を OS のロック画面や通知に出し、そこからの操作を受ける（docs/web.md の「再生」）。
 * 受けるのは再生、一時停止、前後の曲、シーク。10 秒の戻しと送りは受けない。
 */
export function usePlayerMediaSession(el: Ref<HTMLAudioElement | undefined>) {
  if (!import.meta.client || !('mediaSession' in navigator)) {
    return
  }
  const session = navigator.mediaSession
  const { state, current, canPrevious, canNext, toggle, next, previous, seek } = usePlayer()

  watchEffect(() => {
    const song = current.value
    session.metadata = song
      ? new MediaMetadata({
          title: song.title,
          artist: song.artist ?? '',
          album: song.album ?? '',
          artwork: song.coverArt
            ? ARTWORK_SIZES.map(size => ({ src: coverArtUrl(song.coverArt!, size), sizes: `${size}x${size}` }))
            : [],
        })
      : null
  })

  watchEffect(() => {
    session.playbackState = !current.value ? 'none' : state.value.playing ? 'playing' : 'paused'
  })

  /** 押せないときはハンドラを外し、OS にボタンを出させない。 */
  function handle(action: MediaSessionAction, handler: MediaSessionActionHandler | null) {
    try {
      session.setActionHandler(action, handler)
    }
    catch {
      // 対応しない操作を渡すと投げるブラウザがある
    }
  }

  handle('play', () => el.value?.paused && toggle())
  handle('pause', () => el.value?.pause())
  handle('seekto', (details) => {
    if (details.seekTime !== undefined) {
      seek(details.seekTime)
    }
  })
  handle('seekbackward', null)
  handle('seekforward', null)
  watchEffect(() => handle('previoustrack', canPrevious.value ? previous : null))
  watchEffect(() => handle('nexttrack', canNext.value ? next : null))

  /** ロック画面の進み具合のバーを合わせる。長さが分からない間は出さない。 */
  function updatePosition() {
    const audio = el.value
    if (!audio || !('setPositionState' in session)) {
      return
    }
    if (!Number.isFinite(audio.duration) || audio.duration <= 0) {
      session.setPositionState()
      return
    }
    session.setPositionState({
      duration: audio.duration,
      playbackRate: audio.playbackRate || 1,
      position: Math.min(Math.max(0, audio.currentTime), audio.duration),
    })
  }
  for (const event of ['durationchange', 'seeked', 'play', 'pause', 'ratechange', 'emptied']) {
    useEventListener(el, event, updatePosition)
  }

  onBeforeUnmount(() => {
    session.metadata = null
    session.playbackState = 'none'
    for (const action of ['play', 'pause', 'seekto', 'previoustrack', 'nexttrack'] as const) {
      handle(action, null)
    }
  })
}

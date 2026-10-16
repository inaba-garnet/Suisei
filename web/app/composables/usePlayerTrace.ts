import { useEventListener } from '@vueuse/core'

/** 曲の切れ目で止まる理由を切り分けるのに見る `<audio>` の event（#143）。 */
const TRACED_EVENTS = ['ended', 'emptied', 'loadstart', 'canplay', 'play', 'playing', 'pause', 'waiting', 'stalled', 'error'] as const

/**
 * 開発モードのときだけ、`<audio>` の event とタブの表示の切り替わりをコンソールに残す（docs/web.md の「開発」）。
 * Android の実機を `chrome://inspect` でつなぎ、バックグラウンドで曲の切れ目を越えたときの順を見るため。
 */
export function usePlayerTrace(el: Ref<HTMLAudioElement | undefined>) {
  if (!import.meta.client) {
    return
  }
  const dev = useDev()
  function trace(type: string) {
    if (dev.value) {
      console.info('[player]', type, { visibility: document.visibilityState, src: el.value?.currentSrc, time: el.value?.currentTime })
    }
  }
  useEventListener(el, [...TRACED_EVENTS], (e: Event) => trace(e.type))
  useEventListener(document, 'visibilitychange', () => trace('visibilitychange'))
}

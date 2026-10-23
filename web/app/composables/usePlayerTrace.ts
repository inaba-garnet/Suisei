import { useEventListener } from '@vueuse/core'

/** 曲の切れ目で止まる理由を切り分けるのに見る `<audio>` の event（#143）。 */
const TRACED_EVENTS = ['ended', 'emptied', 'loadstart', 'canplay', 'canplaythrough', 'play', 'playing', 'pause', 'waiting', 'stalled', 'error'] as const

/**
 * 開発モードのときだけ、`<audio>` の event とタブの表示の切り替わりをコンソールに残す（docs/web.md の「開発」）。
 * Android の実機を `chrome://inspect` でつなぎ、バックグラウンドで曲の切れ目を越えたときの順を見るため。
 * `<audio>` は二つあるので、何番目か（`a`、`b`）と鳴らしている方か（`active`）を添える。
 */
export function usePlayerTrace(els: Ref<HTMLAudioElement | undefined>[]) {
  if (!import.meta.client) {
    return
  }
  const dev = useDev()
  const active = useActiveAudio()
  function trace(type: string, el?: HTMLAudioElement, label?: string) {
    if (dev.value) {
      console.info('[player]', type, {
        el: label,
        active: el !== undefined && el === active.value,
        visibility: document.visibilityState,
        src: el?.currentSrc,
        time: el?.currentTime,
        error: el?.error?.code,
      })
    }
  }
  els.forEach((el, i) => {
    const label = String.fromCharCode(97 + i)
    useEventListener(el, [...TRACED_EVENTS], (e: Event) => trace(e.type, el.value, label))
  })
  useEventListener(document, 'visibilitychange', () => trace('visibilitychange', active.value))
}

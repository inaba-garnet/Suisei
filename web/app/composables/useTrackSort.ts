import { useLocalStorage } from '@vueuse/core'
import { trackSort } from '~/utils/tracks'

/**
 * 曲の一覧の並び順（docs/web.md の「一覧」）。選んだ順は `storageKey` で残す。
 * 切り替えたときは、残した位置を使わずに先頭から出す。
 */
export function useTrackSort(storageKey: string) {
  const stored = useLocalStorage(storageKey, 'title')
  const sort = computed({
    get: () => trackSort(stored.value),
    set: (value) => {
      stored.value = value
    },
  })

  const popped = usePopped()
  const scroller = useScroller()
  watch(sort, () => {
    popped.value = false
    if (scroller.value) {
      scroller.value.scrollTop = 0
    }
  })

  return sort
}

import { useLocalStorage } from '@vueuse/core'

/**
 * 一覧の並び順（docs/web.md の「一覧」）。選んだ順は `storageKey` で残し、読めない値は `parse` で既定に直す。
 * 切り替えたときは、残した位置を使わずに先頭から出す。
 */
export function useStoredSort<T extends string>(storageKey: string, parse: (value: unknown) => T) {
  const stored = useLocalStorage<string>(storageKey, parse(undefined))
  const sort = computed<T>({
    get: () => parse(stored.value),
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

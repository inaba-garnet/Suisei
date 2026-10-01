/**
 * 一覧の表示の選択（docs/web.md の「一覧」）。`key` で `useState` に持ち、「戻る」で戻ったときは保ち、
 * 開き直したときは `initial` に戻す。切り替えたときは、残した位置を使わずに先頭から出す。
 */
export function useListOption<T>(key: string, initial: T) {
  const value = useState<T>(key, () => initial)
  const popped = usePopped()
  if (!popped.value) {
    value.value = initial
  }
  const scroller = useScroller()
  watch(value, () => {
    popped.value = false
    if (scroller.value) {
      scroller.value.scrollTop = 0
    }
  })
  return value
}

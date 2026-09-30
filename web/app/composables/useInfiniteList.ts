/** 残しておく一覧の状態。 */
interface ListState {
  items: unknown[]
  done: boolean
  scrollTop: number
}

/**
 * `pageSize` 件ずつ読み足す一覧（docs/web.md の「一覧」）。
 * 離れるときに読み込み済みの項目と位置を `key` で残し、「戻る」で戻ったときだけ取り直さずに位置を戻す。
 */
export function useInfiniteList<T>(
  key: string,
  fetchPage: (offset: number, size: number) => Promise<T[]>,
  pageSize = 100,
) {
  const lists = useState<Record<string, ListState>>('lists', () => ({}))
  const popped = usePopped()
  const scroller = useScroller()

  const saved = popped.value ? lists.value[key] : undefined
  const items = shallowRef<T[]>((saved?.items as T[] | undefined) ?? [])
  const done = ref(saved?.done ?? false)
  const loading = ref(false)
  const error = ref<unknown>()

  async function loadMore() {
    if (loading.value || done.value) {
      return
    }
    loading.value = true
    error.value = undefined
    try {
      const page = await fetchPage(items.value.length, pageSize)
      items.value = [...items.value, ...page]
      done.value = page.length < pageSize
    }
    catch (err) {
      error.value = err
    }
    finally {
      loading.value = false
    }
  }

  onMounted(async () => {
    if (!saved) {
      await loadMore()
      return
    }
    // グリッドの幅を測るまでは高さが足りず、位置が頭打ちになる。届くまで数フレーム試す
    await nextTick()
    let frames = 0
    const restore = () => {
      const el = scroller.value
      if (!el) {
        return
      }
      el.scrollTop = saved.scrollTop
      if (Math.abs(el.scrollTop - saved.scrollTop) > 1 && ++frames < 30) {
        requestAnimationFrame(restore)
      }
    }
    requestAnimationFrame(restore)
  })

  onBeforeRouteLeave(() => {
    lists.value[key] = { items: items.value, done: done.value, scrollTop: scroller.value?.scrollTop ?? 0 }
  })

  return { items, done, loading, error, loadMore }
}

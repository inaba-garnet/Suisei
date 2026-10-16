/** 頭文字と、その先頭の位置。 */
export interface Heading {
  heading: string
  offset: number
}

/** `GET /api/headings` の応答（docs/schema.md の「日本語の並べ替え」）。 */
export interface HeadingIndex {
  headings: Heading[]
  total: number
}

/**
 * 頭文字ごとの先頭の位置（`GET /api/headings`）を読む関数を返す。
 * 401 ならセッションが切れたとみなし、ログインの画面に移す（docs/web.md の「一覧」と同じ扱い）。
 */
export function useHeadingIndex() {
  const user = useUser()
  const route = useRoute()

  return async function load(query: Record<string, string>): Promise<HeadingIndex> {
    try {
      return await $fetch<HeadingIndex>('/api/headings', { query })
    }
    catch (err) {
      if (httpStatusOf(err) === 401) {
        user.value = null
        await navigateTo({ path: '/login', query: { redirect: route.fullPath } })
      }
      throw err
    }
  }
}

/** 残しておく一覧の状態。 */
interface IndexedState {
  items: unknown[]
  headings: Heading[]
  loaded: number[]
  scrollTop: number
}

/**
 * 頭文字の索引で、まだ読んでいない位置へ飛べる一覧（docs/web.md の「一覧」）。
 * 先に全体の件数と頭文字ごとの先頭の位置を読み、まだ読んでいない行は `undefined` で仮に置く。
 * `ensure` で見えた範囲を渡すと、その範囲の `pageSize` 件ごとのページを読む。
 * 離れるときに読み込み済みの項目と位置を `key` で残し、「戻る」で戻ったときだけ取り直さずに位置を戻す。
 */
export function useIndexedList<T>(
  key: string,
  loadIndex: () => Promise<HeadingIndex>,
  fetchPage: (offset: number, size: number) => Promise<T[]>,
  pageSize = 100,
) {
  const lists = useState<Record<string, IndexedState>>('indexed-lists', () => ({}))
  const popped = usePopped()
  const scroller = useScroller()

  const saved = popped.value ? lists.value[key] : undefined
  const items = shallowRef<(T | undefined)[]>((saved?.items as (T | undefined)[] | undefined) ?? [])
  const headings = shallowRef<Heading[]>(saved?.headings ?? [])
  const ready = ref(!!saved)
  const error = ref<unknown>()
  const loaded = new Set<number>(saved?.loaded)
  const pending = reactive(new Set<number>())
  const indexing = ref(false)
  const loading = computed(() => indexing.value || pending.size > 0)
  let lastRange: [number, number] = [0, pageSize]

  async function loadPage(page: number) {
    pending.add(page)
    try {
      const rows = await fetchPage(page * pageSize, pageSize)
      const next = [...items.value]
      rows.forEach((row, i) => {
        next[page * pageSize + i] = row
      })
      // 読み込んだ後に曲が消えて、末尾のページが短くなったときは詰める
      if (rows.length < pageSize && page * pageSize + rows.length < next.length) {
        next.length = page * pageSize + rows.length
      }
      items.value = next
      loaded.add(page)
    }
    catch (err) {
      error.value = err
    }
    finally {
      pending.delete(page)
    }
  }

  /** `start` 番目から `end` 番目の手前までの行を読む。失敗した後は `retry` まで読まない。 */
  function ensure(start: number, end: number) {
    lastRange = [start, end]
    if (!ready.value || error.value) {
      return
    }
    const last = Math.min(end, items.value.length) - 1
    for (let page = Math.floor(Math.max(0, start) / pageSize); page * pageSize <= last; page++) {
      if (!loaded.has(page) && !pending.has(page)) {
        loadPage(page)
      }
    }
  }

  async function init() {
    indexing.value = true
    error.value = undefined
    try {
      const index = await loadIndex()
      headings.value = index.headings
      items.value = Array.from<T | undefined>({ length: index.total })
      ready.value = true
    }
    catch (err) {
      error.value = err
    }
    finally {
      indexing.value = false
    }
    ensure(...lastRange)
  }

  function retry() {
    if (!ready.value) {
      init()
      return
    }
    error.value = undefined
    ensure(...lastRange)
  }

  onMounted(async () => {
    if (!saved) {
      await init()
      return
    }
    await nextTick()
    restoreScroll(scroller, saved.scrollTop)
  })

  onBeforeRouteLeave(() => {
    lists.value[key] = { items: items.value, headings: headings.value, loaded: [...loaded], scrollTop: scroller.value?.scrollTop ?? 0 }
  })

  return { items, headings, ready, loading, error, ensure, retry }
}

/**
 * 索引の読み方（`loadIndex`）があれば `useIndexedList` で、なければ `useInfiniteList` で読む一覧。
 * どちらでも同じ形で扱えるようにまとめる。`more` は末尾に近づいたら読み足すか、`ensure` は見えた範囲を読むのに使う。
 */
export function useBrowseList<T>(
  key: string,
  fetchPage: (offset: number, size: number) => Promise<T[]>,
  loadIndex?: () => Promise<HeadingIndex>,
) {
  const indexed = loadIndex ? useIndexedList<T>(key, loadIndex, fetchPage) : undefined
  const infinite = loadIndex ? undefined : useInfiniteList<T>(key, fetchPage)

  return {
    items: computed<(T | undefined)[]>(() => indexed?.items.value ?? infinite!.items.value),
    headings: computed(() => indexed?.headings.value ?? []),
    indexed: !!indexed,
    // 索引のある一覧は、まだ読んでいない行を仮の行で見せるので、索引を読む間だけ読み込み中にする
    loading: computed(() => indexed ? !indexed.ready.value && indexed.loading.value : infinite!.loading.value),
    error: computed(() => (indexed ?? infinite!).error.value),
    done: computed(() => indexed ? indexed.ready.value : infinite!.done.value),
    more: computed(() => !!infinite && !infinite.done.value && !infinite.loading.value && !infinite.error.value),
    loadMore: () => infinite?.loadMore(),
    ensure: (start: number, end: number) => indexed?.ensure(start, end),
    retry: () => indexed ? indexed.retry() : infinite!.loadMore(),
  }
}

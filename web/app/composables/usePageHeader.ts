import { useElementSize } from '@vueuse/core'

/** 画面の上端に残る見出しの帯（PageHeader）と、その高さ。帯の下に一覧の行を送るのに使う。 */
export function usePageHeader() {
  const scroller = useScroller()
  const header = shallowRef<HTMLElement>()
  onMounted(() => {
    header.value = scroller.value?.querySelector<HTMLElement>('[data-testid="page-header"]') ?? undefined
  })
  const { height } = useElementSize(header, undefined, { box: 'border-box' })
  return { header, height }
}
